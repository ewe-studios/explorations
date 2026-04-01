---
title: "Neodatabase Rust Revision"
subtitle: "Graph databases in Rust - neo4j drivers, petgraph, and native implementations"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: 02-query-execution-deep-dive.md
---

# Rust Revision: Neodatabase

## Overview

This document covers graph database implementations and drivers in Rust - connecting to Neo4j, native Rust graph libraries, and building graph applications.

## Part 1: Neo4j Rust Driver

### Bolt Protocol Client

```rust
use neo4rs::{ConfigBuilder, Graph, Node, Path, Relationship};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Person {
    id: u64,
    name: String,
    age: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo4j
    let graph = Graph::new(
        "neo4j://localhost:7687",
        "neo4j",
        "password",
    ).await?;

    // Simple query
    let mut result = graph.execute(
        neo4rs::query("MATCH (p:Person {name: $name}) RETURN p.name, p.age")
            .param("name", "Alice"),
    ).await?;

    while let Some(row) = result.next().await? {
        let name: &str = row.get("p.name")?;
        let age: Option<i32> = row.get("p.age")?;
        println!("{} is {} years old", name, age.unwrap_or(0));
    }

    Ok(())
}
```

### CRUD Operations

```rust
use neo4rs::{query, Graph, Node};

async fn create_person(graph: &Graph, name: &str, age: i32) -> Result<(), Box<dyn std::error::Error>> {
    // CREATE
    graph.run(
        query("CREATE (p:Person {name: $name, age: $age})")
            .param("name", name)
            .param("age", age),
    ).await?;
    Ok(())
}

async fn find_person(graph: &Graph, name: &str) -> Result<Option<Person>, Box<dyn std::error::Error>> {
    // READ
    let mut result = graph.execute(
        query("MATCH (p:Person {name: $name}) RETURN p")
            .param("name", name),
    ).await?;

    if let Some(row) = result.next().await? {
        let node: Node = row.get("p")?;
        Ok(Some(Person {
            id: node.id(),
            name: node.get("name")?,
            age: node.get("age")?,
        }))
    } else {
        Ok(None)
    }
}

async fn update_person_age(graph: &Graph, name: &str, age: i32) -> Result<(), Box<dyn std::error::Error>> {
    // UPDATE
    graph.run(
        query("MATCH (p:Person {name: $name}) SET p.age = $age")
            .param("name", name)
            .param("age", age),
    ).await?;
    Ok(())
}

async fn delete_person(graph: &Graph, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // DELETE
    graph.run(
        query("MATCH (p:Person {name: $name}) DETACH DELETE p")
            .param("name", name),
    ).await?;
    Ok(())
}
```

### Relationship Operations

```rust
use neo4rs::{query, Graph, Relationship};

async fn create_friendship(
    graph: &Graph,
    person1: &str,
    person2: &str,
    since: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // MERGE relationship (idempotent)
    graph.run(
        query(
            "MATCH (p1:Person {name: $name1})
             MATCH (p2:Person {name: $name2})
             MERGE (p1)-[r:FRIENDS_WITH {since: $since}]->(p2)
             RETURN r",
        )
        .param("name1", person1)
        .param("name2", person2)
        .param("since", since),
    ).await?;
    Ok(())
}

async fn find_friends_of_friends(
    graph: &Graph,
    person: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Variable-length pattern
    let mut result = graph.execute(
        query(
            "MATCH (p:Person {name: $name})-[:FRIENDS_WITH*2]->(fof:Person)
             RETURN DISTINCT fof.name",
        )
        .param("name", person),
    ).await?;

    let mut friends = Vec::new();
    while let Some(row) = result.next().await? {
        let name: String = row.get("fof.name")?;
        friends.push(name);
    }

    Ok(friends)
}

async fn find_shortest_path(
    graph: &Graph,
    person1: &str,
    person2: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Shortest path
    let mut result = graph.execute(
        query(
            "MATCH path = shortestPath(
                (p1:Person {name: $name1})-[:FRIENDS_WITH*]-(p2:Person {name: $name2})
             )
             RETURN [n IN nodes(path) | n.name] AS names",
        )
        .param("name1", person1)
        .param("name2", person2),
    ).await?;

    if let Some(row) = result.next().await? {
        let names: Vec<String> = row.get("names")?;
        Ok(names)
    } else {
        Ok(Vec::new())
    }
}
```

### Transaction Support

```rust
use neo4rs::{Graph, Transaction};

async fn transfer_money(
    graph: &Graph,
    from: &str,
    to: &str,
    amount: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Transaction with ACID guarantees
    let mut tx = graph.start_transaction().await?;

    // Debit source account
    tx.run(
        query(
            "MATCH (a:Account {name: $from})
             SET a.balance = a.balance - $amount",
        )
        .param("from", from)
        .param("amount", amount),
    ).await?;

    // Credit destination account
    tx.run(
        query(
            "MATCH (a:Account {name: $to})
             SET a.balance = a.balance + $amount",
        )
        .param("to", to)
        .param("amount", amount),
    ).await?;

    // Create transaction record
    tx.run(
        query(
            "MATCH (from:Account {name: $from})
             MATCH (to:Account {name: $to})
             CREATE (from)-[t:TRANSFERRED {
                 amount: $amount,
                 timestamp: datetime()
             }]->(to)",
        )
        .param("from", from)
        .param("to", to)
        .param("amount", amount),
    ).await?;

    // Commit or rollback
    tx.commit().await?;

    Ok(())
}

async fn with_rollback_example(
    graph: &Graph,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = graph.start_transaction().await?;

    match execute_business_logic(&mut tx).await {
        Ok(_) => tx.commit().await?,
        Err(e) => {
            tx.rollback().await?;
            return Err(e);
        }
    }

    Ok(())
}

async fn execute_business_logic(
    tx: &mut Transaction,
) -> Result<(), Box<dyn std::error::Error>> {
    // ... business logic here
    Ok(())
}
```

## Part 2: Native Rust Graph Libraries

### Petgraph

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{dijkstra, astar};
use petgraph::visit::EdgeRef;

// Define graph with node/edge weights
type SocialGraph = DiGraph<Person, Friendship>;

#[derive(Debug, Clone)]
struct Person {
    id: u64,
    name: String,
    age: i32,
}

#[derive(Debug, Clone)]
struct Friendship {
    since: String,
    closeness: f32,
}

fn main() {
    let mut graph = SocialGraph::new();

    // Add nodes
    let alice = graph.add_node(Person {
        id: 1,
        name: "Alice".to_string(),
        age: 30,
    });

    let bob = graph.add_node(Person {
        id: 2,
        name: "Bob".to_string(),
        age: 25,
    });

    let carol = graph.add_node(Person {
        id: 3,
        name: "Carol".to_string(),
        age: 35,
    });

    // Add edges (directed)
    graph.add_edge(alice, bob, Friendship {
        since: "2020-01-15".to_string(),
        closeness: 0.9,
    });

    graph.add_edge(bob, carol, Friendship {
        since: "2019-06-20".to_string(),
        closeness: 0.8,
    });

    graph.add_edge(alice, carol, Friendship {
        since: "2021-03-10".to_string(),
        closeness: 0.7,
    });

    // BFS traversal
    use petgraph::visit::Bfs;
    let mut bfs = Bfs::new(&graph, alice);
    while let Some(node) = bfs.next(&graph) {
        println!("Visited: {}", graph[node].name);
    }

    // DFS traversal
    use petgraph::visit::Dfs;
    let mut dfs = Dfs::new(&graph, alice);
    while let Some(node) = dfs.next(&graph) {
        println!("DFS: {}", graph[node].name);
    }
}
```

### Shortest Path Algorithms

```rust
use petgraph::graph::DiGraph;
use petgraph::algo::{dijkstra, astar, bellman_ford};

fn shortest_path_example() {
    let mut graph = DiGraph::<&str, f64>::new();

    let a = graph.add_node("A");
    let b = graph.add_node("B");
    let c = graph.add_node("C");
    let d = graph.add_node("D");

    // Weighted edges
    graph.add_edge(a, b, 1.0);
    graph.add_edge(a, c, 4.0);
    graph.add_edge(b, c, 2.0);
    graph.add_edge(b, d, 5.0);
    graph.add_edge(c, d, 1.0);

    // Dijkstra's algorithm (single-source shortest path)
    let distances = dijkstra(&graph, a, None, |e| *e.weight());

    println!("Distances from A:");
    for (node, dist) in &distances {
        println!("  {:?}: {}", graph[*node], dist);
    }
    // A: 0, B: 1, C: 3, D: 4

    // A* algorithm (with heuristic)
    let heuristic = |node: NodeIndex| -> f64 {
        // Euclidean distance to goal (if coordinates known)
        0.0  // Zero heuristic = Dijkstra
    };

    let path = astar(&graph, a, |finish| finish == d, |e| *e.weight(), heuristic);

    if let Some((cost, path)) = path {
        println!("Shortest path cost: {}", cost);
        println!("Path: {:?}", path.iter().map(|n| graph[*n]).collect::<Vec<_>>());
    }

    // Bellman-Ford (handles negative weights)
    let result = bellman_ford::bellman_ford(&graph, a);
    match result {
        Ok(distances) => println!("Bellman-Ford distances: {:?}", distances),
        Err(_) => println!("Negative cycle detected!"),
    }
}
```

### Graph Algorithms

```rust
use petgraph::algo::{kosaraju_scc, tarjan_scc, toposort};
use petgraph::visit::{DfsPostOrder, Reversed};

// Strongly Connected Components
fn find_sccs() {
    let mut graph = DiGraph::<u32, ()>::new();
    let nodes: Vec<_> = (0..8).map(|i| graph.add_node(i)).collect();

    // Create SCCs
    graph.add_edge(nodes[0], nodes[1], ());
    graph.add_edge(nodes[1], nodes[2], ());
    graph.add_edge(nodes[2], nodes[0], ());  // SCC: 0, 1, 2

    graph.add_edge(nodes[3], nodes[4], ());
    graph.add_edge(nodes[4], nodes[5], ());
    graph.add_edge(nodes[5], nodes[3], ());  // SCC: 3, 4, 5

    graph.add_edge(nodes[2], nodes[3], ());  // Cross-SCC edge

    // Kosaraju's algorithm
    let sccs = kosaraju_scc(&graph);
    println!("SCCs (Kosaraju): {:?}", sccs);

    // Tarjan's algorithm
    let sccs = tarjan_scc(&graph);
    println!("SCCs (Tarjan): {:?}", sccs);
}

// Topological Sort (for DAGs)
fn topological_sort_example() {
    let mut graph = DiGraph::<&str, ()>::new();

    let tasks = [
        graph.add_node("compile"),
        graph.add_node("test"),
        graph.add_node("lint"),
        graph.add_node("deploy"),
    ];

    // Dependencies
    graph.add_edge(tasks[0], tasks[1], ());  // compile -> test
    graph.add_edge(tasks[0], tasks[2], ());  // compile -> lint
    graph.add_edge(tasks[1], tasks[3], ());  // test -> deploy
    graph.add_edge(tasks[2], tasks[3], ());  // lint -> deploy

    match toposort(&graph, None) {
        Ok(order) => {
            println!("Valid execution order:");
            for node in order {
                println!("  - {}", graph[node]);
            }
        }
        Err(cycle) => {
            println!("Cycle detected! Cannot sort.");
        }
    }
}

// PageRank (simplified implementation)
fn pagerank(graph: &DiGraph<u32, f64>, damping: f64, iterations: usize) -> Vec<f64> {
    let n = graph.node_count();
    let mut scores = vec![1.0 / n as f64; n];

    for _ in 0..iterations {
        let mut new_scores = vec![0.0; n];

        for node in graph.node_indices() {
            // Damping factor contribution
            new_scores[node.index()] += (1.0 - damping) / n as f64;

            // Incoming link contributions
            for edge in graph.edges_directed(node, petgraph::Direction::Incoming) {
                let source = edge.source();
                let out_degree = graph.edges_directed(source, petgraph::Direction::Outgoing).count();
                if out_degree > 0 {
                    new_scores[node.index()] += damping * scores[source.index()] / out_degree as f64;
                }
            }
        }

        scores = new_scores;
    }

    scores
}
```

## Part 3: Graph Application Patterns

### Social Network Service

```rust
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct SocialGraph {
    // Adjacency list representation
    followers: Arc<RwLock<HashMap<u64, HashSet<u64>>>>,  // user_id -> follower_ids
    following: Arc<RwLock<HashMap<u64, HashSet<u64>>>>,  // user_id -> following_ids
}

impl SocialGraph {
    pub fn new() -> Self {
        Self {
            followers: Arc::new(RwLock::new(HashMap::new())),
            following: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn follow(&self, follower_id: u64, followee_id: u64) {
        let mut followers = self.followers.write().await;
        let mut following = self.following.write().await;

        followers
            .entry(followee_id)
            .or_insert_with(HashSet::new)
            .insert(follower_id);

        following
            .entry(follower_id)
            .or_insert_with(HashSet::new)
            .insert(followee_id);
    }

    pub async fn unfollow(&self, follower_id: u64, followee_id: u64) {
        let mut followers = self.followers.write().await;
        let mut following = self.following.write().await;

        if let Some(followers_set) = followers.get_mut(&followee_id) {
            followers_set.remove(&follower_id);
        }

        if let Some(following_set) = following.get_mut(&follower_id) {
            following_set.remove(&followee_id);
        }
    }

    pub async fn get_followers(&self, user_id: u64) -> HashSet<u64> {
        let followers = self.followers.read().await;
        followers.get(&user_id).cloned().unwrap_or_default()
    }

    pub async fn get_following(&self, user_id: u64) -> HashSet<u64> {
        let following = self.following.read().await;
        following.get(&user_id).cloned().unwrap_or_default()
    }

    /// Friends of friends (2-hop traversal)
    pub async fn get_fof(&self, user_id: u64) -> HashSet<u64> {
        let following = self.get_following(user_id).await;
        let mut fof = HashSet::new();

        for followee_id in &following {
            let second_degree = self.get_following(*followee_id).await;
            for fof_id in second_degree {
                // Exclude self and direct following
                if fof_id != user_id && !following.contains(&fof_id) {
                    fof.insert(fof_id);
                }
            }
        }

        fof
    }

    /// People you may know (ranked by mutual connections)
    pub async fn people_you_may_know(&self, user_id: u64, limit: usize) -> Vec<(u64, usize)> {
        let fof = self.get_fof(user_id).await;
        let following = self.get_following(user_id).await;

        let mut candidates: HashMap<u64, usize> = HashMap::new();

        // Count mutual connections
        for fof_id in fof {
            let fof_followers = self.get_followers(fof_id).await;
            let mutual_count = fof_followers.intersection(&following).count();
            candidates.entry(fof_id).or_insert(0);
            *candidates.get_mut(&fof_id).unwrap() += mutual_count;
        }

        // Sort by mutual count
        let mut sorted: Vec<_> = candidates.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);

        sorted
    }
}
```

### Recommendation Engine

```rust
use std::collections::HashMap;

/// Collaborative filtering using graph traversal
pub struct RecommendationEngine {
    // user_id -> item_ids they've interacted with
    user_items: HashMap<u64, HashSet<u64>>,
    // item_id -> user_ids who've interacted with it
    item_users: HashMap<u64, HashSet<u64>>,
}

impl RecommendationEngine {
    pub fn new() -> Self {
        Self {
            user_items: HashMap::new(),
            item_users: HashMap::new(),
        }
    }

    pub fn record_interaction(&mut self, user_id: u64, item_id: u64) {
        self.user_items
            .entry(user_id)
            .or_insert_with(HashSet::new)
            .insert(item_id);

        self.item_users
            .entry(item_id)
            .or_insert_with(HashSet::new)
            .insert(user_id);
    }

    /// Item-based collaborative filtering
    /// "Users who liked X also liked Y"
    pub fn recommend_items(&self, user_id: u64, limit: usize) -> Vec<(u64, f32)> {
        let user_history = match self.user_items.get(&user_id) {
            Some(items) => items,
            None => return Vec::new(),
        };

        // Count co-occurrences
        let mut item_scores: HashMap<u64, f32> = HashMap::new();

        for &history_item in user_history {
            if let Some(similar_users) = self.item_users.get(&history_item) {
                for &similar_user in similar_users {
                    if let Some(similar_items) = self.user_items.get(&similar_user) {
                        for &similar_item in similar_items {
                            if !user_history.contains(&similar_item) {
                                *item_scores.entry(similar_item).or_insert(0.0) += 1.0;
                            }
                        }
                    }
                }
            }
        }

        // Normalize and sort
        let mut scored: Vec<_> = item_scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(limit);

        scored
    }

    /// User-based collaborative filtering
    /// "Users similar to you also liked..."
    pub fn find_similar_users(&self, user_id: u64, limit: usize) -> Vec<(u64, f32)> {
        let user_history = match self.user_items.get(&user_id) {
            Some(items) => items,
            None => return Vec::new(),
        };

        let mut user_scores: HashMap<u64, f32> = HashMap::new();

        // Find users who share item interactions
        for &item in user_history {
            if let Users) {
                for &other_user in users {
                    if other_user != user_id {
                        *user_scores.entry(other_user).or_insert(0.0) += 1.0;
                    }
                }
            }
        }

        // Jaccard similarity
        for (other_user, score) in &mut user_scores {
            if let Some(other_history) = self.user_items.get(other_user) {
                let intersection = user_history.intersection(other_history).count();
                let union = user_history.union(other_history).count();
                *score = if union > 0 {
                    intersection as f32 / union as f32
                } else {
                    0.0
                };
            }
        }

        let mut scored: Vec<_> = user_scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(limit);

        scored
    }
}
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
