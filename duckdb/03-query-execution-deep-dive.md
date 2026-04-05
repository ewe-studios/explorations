---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb/src/execution/
explored_at: 2026-04-04
focus: Vectorized execution, query optimization, parallel processing, statistics-based pruning
---

# Deep Dive: Query Execution and Optimization

## Overview

This deep dive examines DuckDB's query execution engine, covering vectorized processing, operator implementation, query optimization strategies, parallel execution, and statistics-based pruning. We explore how DuckDB achieves high-performance analytical queries.

## Architecture

```mermaid
flowchart TB
    subgraph Parser
        SQL[SQL Query] --> Parser[Parser]
        Parser --> AST[Abstract Syntax Tree]
    end
    
    subgraph Binder
        AST --> Binder[Binder]
        Binder --> Catalog[Catalog Lookup]
        Binder --> BT[Bound Tree]
    end
    
    subgraph Optimizer
        BT --> Optimizer[Optimizer]
        Optimizer --> Stats[Statistics Collector]
        Optimizer --> OP[Optimized Plan]
    end
    
    subgraph Execution
        OP --> Pipeline[Pipeline Builder]
        Pipeline --> P1[Pipeline 1]
        Pipeline --> P2[Pipeline 2]
        P1 --> OPipe[Operator Pipeline]
        P2 --> OPipe
        OPipe --> VE[Vectorized Executor]
    end
    
    subgraph Storage
        VE --> BM[Buffer Manager]
        VE --> CC[Column Chunk]
        CC --> RG[Row Group]
    end
    
    VE --> Result[Result]
```

## Vectorized Execution Model

### Vector Structure

```cpp
// src/include/vector.hpp

class Vector {
private:
    PhysicalType type;           // Data type
    idx_t capacity;              // Vector capacity (STANDARD_VECTOR_SIZE)
    idx_t size;                  // Current size
    data_ptr_t data;             // Pointer to data
    validity_mask_t *validity;   // Validity mask (nulls)
    vector<unique_ptr<Vector>> children; // For nested types
    
public:
    static constexpr idx_t STANDARD_VECTOR_SIZE = 2048;
    
    Vector(PhysicalType type, idx_t capacity = STANDARD_VECTOR_SIZE);
    
    /// Get value at index
    template <typename T>
    T GetValue(idx_t index) const {
        assert(index < size);
        if (validity && !validity[index]) {
            throw NullValueException();
        }
        return reinterpret_cast<T*>(data)[index];
    }
    
    /// Set value at index
    template <typename T>
    void SetValue(idx_t index, T value) {
        assert(index < size);
        reinterpret_cast<T*>(data)[index] = value;
        if (validity) {
            validity[index] = true;
        }
    }
    
    /// Set null at index
    void SetNull(idx_t index) {
        if (validity) {
            validity[index] = false;
        }
    }
    
    /// Check if null at index
    bool IsNull(idx_t index) const {
        return validity && !validity[index];
    }
    
    /// Get writable data pointer
    template <typename T>
    T* GetData() {
        return reinterpret_cast<T*>(data);
    }
    
    /// Get read-only data pointer
    template <typename T>
    const T* GetData() const {
        return reinterpret_cast<const T*>(data);
    }
    
    /// Uniform value optimization (all values same)
    bool HasUniformValue() const {
        return uniform_value != nullptr;
    }
    
    /// Constant vector optimization
    bool IsConstant() const {
        return constant;
    }
    
private:
    void *uniform_value;  // When all values are identical
    bool constant;        // Is this a constant vector
};

/// Validity mask - 64 bits per word
struct validity_mask_t {
    static constexpr idx_t BLOCK_SIZE = 64;
    
    uint64_t *mask;
    idx_t blocks;
    
    bool Get(idx_t index) const {
        auto block = index / BLOCK_SIZE;
        auto bit = index % BLOCK_SIZE;
        return (mask[block] & (1ULL << bit)) != 0;
    }
    
    void Set(idx_t index, bool valid) {
        auto block = index / BLOCK_SIZE;
        auto bit = index % BLOCK_SIZE;
        if (valid) {
            mask[block] |= (1ULL << bit);
        } else {
            mask[block] &= ~(1ULL << bit);
        }
    }
};
```

### Vectorized Operator Interface

```cpp
// src/include/execution/operator.hpp

class Operator {
public:
    virtual ~Operator() = default;
    
    /// Get next batch of tuples (vectors)
    virtual DataChunk Execute(ExecutionContext &context) = 0;
    
    /// Operator type for explain
    virtual string GetName() const = 0;
    
    /// Get output types
    virtual vector<LogicalType> GetTypes() const = 0;
    
    /// Get cardinality estimate
    virtual idx_t GetCardinality() const = 0;
};

/// Data chunk - collection of vectors (one batch)
struct DataChunk {
    vector<Vector> columns;
    idx_t size;
    
    DataChunk() : size(0) {}
    
    void Initialize(const vector<LogicalType> &types, 
                   idx_t capacity = Vector::STANDARD_VECTOR_SIZE) {
        columns.clear();
        for (const auto &type : types) {
            columns.emplace_back(type.InternalType(), capacity);
        }
        size = 0;
    }
    
    void Reset() {
        size = 0;
        for (auto &col : columns) {
            col.Reset();
        }
    }
    
    void SetSize(idx_t new_size) {
        assert(new_size <= Vector::STANDARD_VECTOR_SIZE);
        size = new_size;
    }
    
    /// Get column by index
    Vector &GetColumn(idx_t index) {
        return columns[index];
    }
    
    /// Get column by index (const)
    const Vector &GetColumn(idx_t index) const {
        return columns[index];
    }
};
```

### Table Scan Operator

```cpp
// src/execution/operator/scan/physical_table_scan.cpp

class PhysicalTableScan : public Operator {
private:
    TableCatalogEntry *table;
    ColumnDataCollection *column_data;
    ColumnDataScanState scan_state;
    vector<idx_t> column_ids;  // IDs of columns to scan
    vector<LogicalType> return_types;
    
public:
    PhysicalTableScan(
        TableCatalogEntry *table,
        vector<idx_t> column_ids,
        vector<LogicalType> return_types
    ) : table(table),
        column_ids(column_ids),
        return_types(return_types) {}
    
    DataChunk Execute(ExecutionContext &context) override {
        DataChunk result;
        result.Initialize(return_types);
        
        // Scan next batch from column data
        auto scan_count = column_data->Scan(
            scan_state,
            column_ids,
            result
        );
        
        if (scan_count == 0) {
            // End of table
            result.size = 0;
        } else {
            result.SetSize(scan_count);
        }
        
        return result;
    }
    
    string GetName() const override {
        return "TableScan";
    }
};

/// Column data collection - manages scanning across row groups
class ColumnDataCollection {
private:
    vector<unique_ptr<RowGroup>> row_groups;
    BufferManager &buffer_manager;
    
public:
    ColumnDataCollection(BufferManager &manager) 
        : buffer_manager(manager) {}
    
    /// Scan from current position
    idx_t Scan(
        ColumnDataScanState &state,
        const vector<idx_t> &column_ids,
        DataChunk &result
    ) {
        idx_t total_count = 0;
        auto &capacity = Vector::STANDARD_VECTOR_SIZE;
        
        while (total_count < capacity) {
            // Get current row group
            auto row_group = GetRowGroup(state.current_row_group);
            if (!row_group) {
                break; // No more data
            }
            
            // Scan from row group
            auto scan_count = row_group->Scan(
                state,
                column_ids,
                result,
                total_count,
                capacity - total_count
            );
            
            total_count += scan_count;
            
            if (scan_count < capacity - total_count) {
                // Row group exhausted, move to next
                state.current_row_group++;
                state.current_row_in_group = 0;
            } else {
                // Batch full
                break;
            }
        }
        
        return total_count;
    }
};
```

### Filter Operator

```cpp
// src/execution/operator/filter/physical_filter.cpp

class PhysicalFilter : public Operator {
private:
    unique_ptr<Expression> condition;
    unique_ptr<Operator> child;
    
public:
    PhysicalFilter(unique_ptr<Expression> condition, 
                   unique_ptr<Operator> child)
        : condition(std::move(condition)),
          child(std::move(child)) {}
    
    DataChunk Execute(ExecutionContext &context) override {
        while (true) {
            // Get batch from child
            auto input = child->Execute(context);
            
            if (input.size == 0) {
                // No more data
                return input;
            }
            
            // Apply filter condition
            auto selection = ApplyFilter(input, condition.get(), context);
            
            if (selection.count > 0) {
                // Some rows passed filter
                return SliceChunk(input, selection);
            }
            // All rows filtered, continue to next batch
        }
    }
    
    /// Apply filter and return selection vector
    SelectionVector ApplyFilter(
        const DataChunk &input,
        Expression *condition,
        ExecutionContext &context
    ) {
        SelectionVector result(input.size);
        result.count = 0;
        
        // Execute condition expression (returns boolean vector)
        Vector condition_vector(LogicalType::BOOLEAN, input.size);
        condition->Execute(input, condition_vector, context);
        
        auto condition_data = condition_vector.GetData<bool>();
        
        // Build selection vector
        for (idx_t i = 0; i < input.size; i++) {
            if (!condition_vector.IsNull(i) && condition_data[i]) {
                result.data[result.count++] = i;
            }
        }
        
        return result;
    }
    
    /// Slice chunk using selection vector
    DataChunk SliceChunk(
        const DataChunk &input,
        const SelectionVector &selection
    ) {
        DataChunk result;
        result.Initialize(input.GetTypes(), selection.count);
        result.SetSize(selection.count);
        
        for (idx_t col = 0; col < input.columns.size(); col++) {
            // Gather based on selection
            GatherVector(
                input.GetColumn(col),
                result.GetColumn(col),
                selection
            );
        }
        
        return result;
    }
};

struct SelectionVector {
    idx_t *data;
    idx_t count;
    idx_t capacity;
    
    SelectionVector(idx_t capacity) 
        : data(new idx_t[capacity]), count(0), capacity(capacity) {}
};
```

### Projection Operator

```cpp
// src/execution/operator/projection/physical_projection.cpp

class PhysicalProjection : public Operator {
private:
    vector<unique_ptr<Expression>> expressions;
    unique_ptr<Operator> child;
    
public:
    PhysicalProjection(
        vector<unique_ptr<Expression>> expressions,
        unique_ptr<Operator> child
    ) : expressions(std::move(expressions)),
        child(std::move(child)) {}
    
    DataChunk Execute(ExecutionContext &context) override {
        // Get batch from child
        auto input = child->Execute(context);
        
        if (input.size == 0) {
            return input;
        }
        
        // Project expressions
        DataChunk result;
        result.Initialize(GetTypes(), input.size);
        result.SetSize(input.size);
        
        for (idx_t i = 0; i < expressions.size(); i++) {
            expressions[i]->Execute(input, result.GetColumn(i), context);
        }
        
        return result;
    }
};
```

### Hash Join Operator

```cpp
// src/execution/operator/join/physical_hash_join.cpp

class PhysicalHashJoin : public Operator {
private:
    vector<unique_ptr<Expression>> conditions;
    unique_ptr<Operator> left;
    unique_ptr<Operator> right;
    JoinType join_type;
    
    // Hash table state
    struct HashJoinState {
        // Build phase
        unique_ptr<HashTable> hash_table;
        bool built;
        
        // Probe phase
        DataChunk left_batch;
        idx_t left_position;
        
        // Output
        vector<idx_t> left_match;
        vector<idx_t> right_match;
        idx_t output_position;
    };
    
    HashJoinState state;
    
public:
    PhysicalHashJoin(
        vector<unique_ptr<Expression>> conditions,
        unique_ptr<Operator> left,
        unique_ptr<Operator> right,
        JoinType join_type
    ) : conditions(std::move(conditions)),
        left(std::move(left)),
        right(std::move(right)),
        join_type(join_type) {}
    
    DataChunk Execute(ExecutionContext &context) override {
        // Build phase (first call only)
        if (!state.built) {
            BuildHashTable(context);
            state.built = true;
        }
        
        // Probe phase
        return ProbeHashTable(context);
    }
    
    /// Build hash table from right side
    void BuildHashTable(ExecutionContext &context) {
        // Determine build columns (from join conditions)
        auto build_columns = GetBuildColumns();
        
        // Create hash table
        state.hash_table = make_unique<HashTable>(
            build_columns.size(),
            context.buffer_manager
        );
        
        // Scan right side and build
        while (true) {
            auto batch = right->Execute(context);
            
            if (batch.size == 0) {
                break;
            }
            
            // Extract build keys
            VectorVector keys;
            for (auto col : build_columns) {
                keys.push_back(&batch.GetColumn(col));
            }
            
            // Insert into hash table
            state.hash_table->Insert(keys, batch.size);
        }
    }
    
    /// Probe hash table against left side
    DataChunk ProbeHashTable(ExecutionContext &context) {
        DataChunk result;
        result.Initialize(GetTypes(), Vector::STANDARD_VECTOR_SIZE);
        idx_t result_count = 0;
        
        while (result_count < Vector::STANDARD_VECTOR_SIZE) {
            // Get next left batch if needed
            if (state.left_position >= state.left_batch.size) {
                state.left_batch = left->Execute(context);
                state.left_position = 0;
                
                if (state.left_batch.size == 0) {
                    // No more left data
                    break;
                }
                
                // Probe batch
                ProbeBatch(state.left_batch);
            }
            
            // Output matches
            while (state.output_position < state.left_match.size() &&
                   result_count < Vector::STANDARD_VECTOR_SIZE) {
                auto left_idx = state.left_match[state.output_position];
                auto right_idx = state.right_match[state.output_position];
                
                // Copy left row
                for (idx_t col = 0; col < state.left_batch.columns.size(); col++) {
                    CopyValue(
                        result.GetColumn(col),
                        result_count,
                        state.left_batch.GetColumn(col),
                        left_idx
                    );
                }
                
                // Copy right row
                // ... similar logic
                
                result_count++;
                state.output_position++;
            }
        }
        
        result.SetSize(result_count);
        return result;
    }
    
    void ProbeBatch(const DataChunk &batch) {
        state.left_match.clear();
        state.right_match.clear();
        state.output_position = 0;
        
        // Get probe keys
        auto probe_columns = GetProbeColumns();
        VectorVector keys;
        for (auto col : probe_columns) {
            keys.push_back(&batch.GetColumn(col));
        }
        
        // Probe hash table
        state.hash_table->Probe(
            keys,
            batch.size,
            state.left_match,
            state.right_match
        );
    }
};

enum class JoinType {
    INNER,
    LEFT,
    RIGHT,
    FULL,
    SEMI,
    ANTI
};
```

## Query Optimization

### Statistics Collection

```cpp
// src/include/statistics/statistics.hpp

class ColumnStatistics {
private:
    LogicalType type;
    
    // Cardinality
    idx_t count;
    idx_t null_count;
    
    // Min/Max
    Value min_value;
    Value max_value;
    
    // Distinct count estimate
    idx_t distinct_count;
    
    // Most common values
    struct CommonValue {
        Value value;
        idx_t frequency;
    };
    vector<CommonValue> common_values;
    
public:
    ColumnStatistics(LogicalType type) : type(type), count(0), null_count(0) {}
    
    /// Check if predicate can be pruned
    bool CanPrune(const Expression &predicate) const {
        if (predicate.type == ExpressionType::COMPARE_LESSTHAN) {
            auto &constant = predicate.GetConstant();
            // If min >= constant, no rows match
            return min_value >= constant;
        }
        
        if (predicate.type == ExpressionType::COMPARE_GREATERTHAN) {
            auto &constant = predicate.GetConstant();
            // If max <= constant, no rows match
            return max_value <= constant;
        }
        
        if (predicate.type == ExpressionType::COMPARE_EQUAL) {
            auto &constant = predicate.GetConstant();
            // If constant outside [min, max], no rows match
            return constant < min_value || constant > max_value;
        }
        
        return false;
    }
    
    /// Get selectivity estimate for predicate
    double GetSelectivity(const Expression &predicate) const {
        if (predicate.type == ExpressionType::COMPARE_EQUAL) {
            // Use distinct count for estimate
            if (distinct_count > 0) {
                return 1.0 / distinct_count;
            }
            return 0.1; // Default 10%
        }
        
        if (predicate.type == ExpressionType::COMPARE_LESSTHAN ||
            predicate.type == ExpressionType::COMPARE_GREATERTHAN) {
            // Assume uniform distribution
            return 0.33; // Default 1/3
        }
        
        return 1.0; // No selectivity
    }
    
    /// Merge statistics from another column
    void Merge(const ColumnStatistics &other) {
        count += other.count;
        null_count += other.null_count;
        
        if (other.min_value < min_value) {
            min_value = other.min_value;
        }
        if (other.max_value > max_value) {
            max_value = other.max_value;
        }
        
        // Update distinct count (HyperLogLog in production)
        distinct_count = std::max(distinct_count, other.distinct_count);
    }
};

class TableStatistics {
private:
    vector<ColumnStatistics> columns;
    idx_t row_count;
    
public:
    TableStatistics(const vector<ColumnStatistics> &cols)
        : columns(cols) {
        if (!cols.empty()) {
            row_count = cols[0].GetCount();
        } else {
            row_count = 0;
        }
    }
    
    idx_t GetRowCount() const { return row_count; }
    const ColumnStatistics &GetColumn(idx_t index) const { 
        return columns[index]; 
    }
};
```

### Predicate Pushdown

```cpp
// src/optimizer/predicate_pushdown.cpp

class PredicatePushdownOptimizer {
public:
    unique_ptr<Operator> Optimize(unique_ptr<Operator> root) {
        vector<unique_ptr<Expression>> predicates;
        return PushDown(std::move(root), predicates);
    }
    
private:
    unique_ptr<Operator> PushDown(
        unique_ptr<Operator> op,
        vector<unique_ptr<Expression>> &predicates
    ) {
        if (auto *filter = dynamic_cast<PhysicalFilter*>(op.get())) {
            // Accumulate filter conditions
            predicates.push_back(std::move(filter->condition));
            return PushDown(std::move(filter->child), predicates);
        }
        
        if (auto *join = dynamic_cast<PhysicalHashJoin*>(op.get())) {
            return PushDownJoin(std::move(join), predicates);
        }
        
        if (auto *scan = dynamic_cast<PhysicalTableScan*>(op.get())) {
            // Push predicates into scan as table filters
            return PushDownScan(std::move(scan), predicates);
        }
        
        // For other operators, rebuild with pushed predicates
        for (auto &child : op->children) {
            child = PushDown(std::move(child), predicates);
        }
        
        // Rebuild filters on top
        if (!predicates.empty()) {
            return BuildFilterChain(std::move(op), predicates);
        }
        
        return op;
    }
    
    unique_ptr<Operator> PushDownJoin(
        unique_ptr<PhysicalHashJoin> join,
        vector<unique_ptr<Expression>> &predicates
    ) {
        // Split predicates into left-only, right-only, and join conditions
        vector<unique_ptr<Expression>> left_preds;
        vector<unique_ptr<Expression>> right_preds;
        vector<unique_ptr<Expression>> join_preds;
        
        for (auto &pred : predicates) {
            auto binding = pred->GetBindings();
            
            if (binding.references_only(join->left->bindings)) {
                left_preds.push_back(std::move(pred));
            } else if (binding.references_only(join->right->bindings)) {
                right_preds.push_back(std::move(pred));
            } else {
                join_preds.push_back(std::move(pred));
            }
        }
        
        // Push down to children
        join->left = PushDown(std::move(join->left), left_preds);
        join->right = PushDown(std::move(join->right), right_preds);
        
        // Keep join predicates as filter on top
        predicates = std::move(join_preds);
        
        return join;
    }
    
    unique_ptr<Operator> PushDownScan(
        unique_ptr<PhysicalTableScan> scan,
        vector<unique_ptr<Expression>> &predicates
    ) {
        // Convert predicates to table filters
        vector<unique_ptr<Expression>> table_filters;
        vector<unique_ptr<Expression>> remaining_filters;
        
        for (auto &pred : predicates) {
            if (IsPushableIntoScan(*pred)) {
                table_filters.push_back(std::move(pred));
            } else {
                remaining_filters.push_back(std::move(pred));
            }
        }
        
        // Add filters to scan
        scan->AddFilters(std::move(table_filters));
        
        // Return remaining filters
        predicates = std::move(remaining_filters);
        
        return scan;
    }
    
    bool IsPushableIntoScan(const Expression &expr) const {
        // Can push down:
        // - Simple comparisons on base columns
        // - AND/OR combinations of pushable expressions
        // - NOT of pushable expressions
        
        // Cannot push down:
        // - Subqueries
        // - Aggregate functions
        // - Window functions
        // - Expressions with side effects
        
        return expr.IsPure() && !expr.HasSubquery();
    }
    
    unique_ptr<Operator> BuildFilterChain(
        unique_ptr<Operator> child,
        vector<unique_ptr<Expression>> &predicates
    ) {
        unique_ptr<Operator> result = std::move(child);
        
        for (auto &pred : predicates) {
            result = make_unique<PhysicalFilter>(
                std::move(pred),
                std::move(result)
            );
        }
        
        predicates.clear();
        return result;
    }
};
```

### Parallel Execution

```cpp
// src/execution/pipeline/parallel_pipeline.cpp

class PipelineExecutor {
private:
    vector<unique_ptr<Operator>> operators;
    vector<unique_ptr<Pipeline>> pipelines;
    ExecutionContext &context;
    idx_t degree_of_parallelism;
    
public:
    PipelineExecutor(
        vector<unique_ptr<Operator>> operators,
        ExecutionContext &context
    ) : operators(std::move(operators)),
        context(context),
        degree_of_parallelism(std::thread::hardware_concurrency()) {}
    
    /// Execute pipeline with parallelism
    DataChunk Execute() {
        // Build execution pipeline
        BuildPipelines();
        
        // Execute with worker threads
        vector<std::thread> workers;
        ConcurrentQueue<DataChunk> output_queue;
        
        for (idx_t i = 0; i < degree_of_parallelism; i++) {
            workers.push_back(std::thread([this, &output_queue, i]() {
                WorkerThread(i, output_queue);
            }));
        }
        
        // Collect results
        DataChunk result;
        result.Initialize(GetOutputTypes());
        
        idx_t total_count = 0;
        DataChunk chunk;
        
        while (output_queue.TryDequeue(chunk)) {
            // Append to result
            for (idx_t col = 0; col < chunk.columns.size(); col++) {
                AppendVector(
                    result.GetColumn(col),
                    chunk.GetColumn(col),
                    total_count
                );
            }
            total_count += chunk.size;
        }
        
        result.SetSize(total_count);
        
        // Wait for workers
        for (auto &worker : workers) {
            worker.join();
        }
        
        return result;
    }
    
private:
    void BuildPipelines() {
        // Split operators into pipeline breakers
        // Pipeline breakers: Hash Join, Order By, Aggregate
        // Between breakers: parallel execution possible
        
        vector<unique_ptr<Operator>> current_pipeline;
        
        for (auto &op : operators) {
            current_pipeline.push_back(std::move(op));
            
            if (IsPipelineBreaker(*current_pipeline.back())) {
                pipelines.push_back(make_unique<Pipeline>(
                    std::move(current_pipeline)
                ));
                current_pipeline.clear();
            }
        }
        
        if (!current_pipeline.empty()) {
            pipelines.push_back(make_unique<Pipeline>(
                std::move(current_pipeline)
            ));
        }
    }
    
    bool IsPipelineBreaker(const Operator &op) const {
        return op.GetType() == OperatorType::HASH_JOIN ||
               op.GetType() == OperatorType::ORDER_BY ||
               op.GetType() == OperatorType::AGGREGATE;
    }
    
    void WorkerThread(
        idx_t thread_id,
        ConcurrentQueue<DataChunk> &output_queue
    ) {
        // Each worker processes a portion of the data
        auto &source = pipelines[0]->GetSource();
        
        while (true) {
            auto chunk = source.Execute(context);
            
            if (chunk.size == 0) {
                break;
            }
            
            // Process through pipeline
            for (idx_t i = 1; i < pipelines.size(); i++) {
                chunk = pipelines[i]->Execute(chunk, context);
            }
            
            output_queue.Enqueue(std::move(chunk));
        }
    }
};

/// Parallel scan state
struct ParallelScanState {
    atomic<idx_t> next_row_group;
    idx_t total_row_groups;
    mutex result_lock;
    
    ParallelScanState(idx_t total) 
        : next_row_group(0), total_row_groups(total) {}
    
    /// Get next work unit (row group) atomically
    bool GetNextWork(idx_t &row_group_out) {
        idx_t current = next_row_group.fetch_add(1);
        if (current >= total_row_groups) {
            return false;
        }
        row_group_out = current;
        return true;
    }
};

/// Parallel table scan
class ParallelTableScan : public Operator {
private:
    ColumnDataCollection *collection;
    vector<idx_t> column_ids;
    vector<LogicalType> return_types;
    ParallelScanState scan_state;
    
public:
    ParallelTableScan(
        ColumnDataCollection *collection,
        vector<idx_t> column_ids,
        vector<LogicalType> return_types
    ) : collection(collection),
        column_ids(column_ids),
        return_types(return_types),
        scan_state(collection->RowCount()) {}
    
    DataChunk Execute(ExecutionContext &context) override {
        idx_t row_group;
        
        // Atomically get next work unit
        if (!scan_state.GetNextWork(row_group)) {
            // No more work
            return DataChunk();
        }
        
        // Scan assigned row group
        DataChunk result;
        result.Initialize(return_types);
        
        auto count = collection->ScanRowGroup(
            row_group,
            column_ids,
            result
        );
        
        result.SetSize(count);
        return result;
    }
};
```

## Conclusion

DuckDB's query execution engine achieves high performance through:

1. **Vectorized Execution**: Process 2048 tuples at once, SIMD optimizations
2. **Selection Vectors**: Avoid data copying during filtering
3. **Predicate Pushdown**: Filter early, reduce data movement
4. **Statistics-Based Pruning**: Skip irrelevant data using min/max
5. **Parallel Execution**: Multiple worker threads, pipeline parallelism
6. **Hash Join Optimization**: Build/probe with vectorized batches
7. **Zero-Copy Operations**: Reuse buffers, minimize allocations
