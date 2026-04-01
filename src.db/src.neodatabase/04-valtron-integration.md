---
title: "Neodatabase Valtron Integration"
subtitle: "Graph-powered applications with Neo4j - social networks, fraud detection, and knowledge graphs"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: production-grade.md
---

# 04 - Valtron Integration: Neodatabase

## Overview

This document covers integrating Neo4j graph database into applications using the Valtron pattern - building social network features, fraud detection systems, and knowledge graph applications.

## Part 1: Social Network Service

### Graph-Based Social Network

```rust
use valtron::{Effect, TaskResult, TaskIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friendship {
    pub since: String,
    pub closeness: f32,
}

#[derive(Debug, Clone)]
pub enum SocialGraphOp {
    CreateUser(User),
    Follow { follower_id: u64, followee_id: u64 },
    Unfollow { follower_id: u64, followee_id: u64 },
    GetFollowers { user_id: u64 },
    GetFollowing { user_id: u64 },
    GetFriendsOfFriends { user_id: u64 },
    GetPeopleYouMayKnow { user_id: u64, limit: usize },
    GetShortestPath { from: u64, to: u64 },
}

pub struct SocialGraphService {
    neo4j_uri: String,
    username: String,
    password: String,
}

impl SocialGraphService {
    pub fn new(neo4j_uri: &str, username: &str, password: &str) -> Self {
        Self {
            neo4j_uri: neo4j_uri.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn create_user(&self, user: User) -> TaskIterator<TaskResult<User>, SocialGraphOp> {
        let query = r#"
            CREATE (u:User {
                id: $id,
                name: $name,
                email: $email,
                created_at: datetime()
            })
            RETURN u
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute CREATE query via Neo4j driver
            // Returns created user
            Ok(user.clone())
        }))
    }

    pub fn follow(
        &self,
        follower_id: u64,
        followee_id: u64,
    ) -> TaskIterator<TaskResult<bool>, SocialGraphOp> {
        let query = r#"
            MATCH (follower:User {id: $follower_id})
            MATCH (followee:User {id: $followee_id})
            MERGE (follower)-[r:FOLLOWS {since: $since}]->(followee)
            RETURN r
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute MERGE query
            // Returns true if relationship created or existed
            Ok(true)
        }))
    }

    pub fn get_followers(&self, user_id: u64) -> TaskIterator<TaskResult<Vec<User>>, SocialGraphOp> {
        let query = r#"
            MATCH (follower:User)-[:FOLLOWS]->(user:User {id: $user_id})
            RETURN follower
            ORDER BY follower.name
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute MATCH query
            // Returns list of follower users
            Ok(vec![])
        }))
    }

    pub fn get_friends_of_friends(
        &self,
        user_id: u64,
    ) -> TaskIterator<TaskResult<Vec<User>>, SocialGraphOp> {
        // 2-hop traversal with deduplication
        let query = r#"
            MATCH (me:User {id: $user_id})-[:FOLLOWS*2]->(fof:User)
            WHERE NOT (me)-[:FOLLOWS]->(fof)
              AND fof <> me
            RETURN DISTINCT fof
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute variable-length pattern match
            // Returns FOFs excluding self and direct follows
            Ok(vec![])
        }))
    }

    pub fn people_you_may_know(
        &self,
        user_id: u64,
        limit: usize,
    ) -> TaskIterator<TaskResult<Vec<(User, usize)>>, SocialGraphOp> {
        // Ranked by mutual connections
        let query = r#"
            MATCH (me:User {id: $user_id})-[:FOLLOWS]->(followee:User)
            MATCH (followee)-[:FOLLOWS]->(fof:User)
            WHERE NOT (me)-[:FOLLOWS]->(fof)
              AND fof <> me
            RETURN fof, count(DISTINCT followee) AS mutual_count
            ORDER BY mutual_count DESC
            LIMIT $limit
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute aggregation query
            // Returns (user, mutual_count) pairs
            Ok(vec![])
        }))
    }

    pub fn get_shortest_path(
        &self,
        from: u64,
        to: u64,
    ) -> TaskIterator<TaskResult<Vec<User>>, SocialGraphOp> {
        let query = r#"
            MATCH path = shortestPath(
                (from:User {id: $from})-[:FOLLOWS*]-(to:User {id: $to})
            )
            RETURN [n IN nodes(path) | n] AS users
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute shortestPath query
            // Returns path as list of users
            Ok(vec![])
        }))
    }
}
```

### Social Network Feed

```rust
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: u64,
    pub author_id: u64,
    pub content: String,
    pub created_at: String,
    pub likes: u32,
}

impl SocialGraphService {
    /// Get personalized feed using graph traversal
    pub fn get_feed(
        &self,
        user_id: u64,
        limit: usize,
    ) -> TaskIterator<TaskResult<Vec<Post>>, SocialGraphOp> {
        // Query posts from followed users and 2nd-degree connections
        let query = r#"
            MATCH (me:User {id: $user_id})

            // Posts from direct follows (weight: 1.0)
            OPTIONAL MATCH (me)-[:FOLLOWS]-(followed:User)
            WITH me, followed, 1.0 AS weight

            UNION

            // Posts from FOFs (weight: 0.5)
            OPTIONAL MATCH (me)-[:FOLLOWS*2]-(fof:User)
            WHERE NOT (me)-[:FOLLOWS]->(fof)
            WITH me, fof, 0.5 AS weight

            // Get posts with weighted ranking
            MATCH (author:User)-[:CREATED]->(post:Post)
            WHERE author IN [followed, fof]
              AND post.created_at > datetime() - duration('P7D')  // Last 7 days
            RETURN post, author, weight
            ORDER BY weight DESC, post.created_at DESC
            LIMIT $limit
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Execute feed query with ranking
            Ok(vec![])
        }))
    }

    /// Get recommended accounts to follow
    pub fn recommend_accounts(
        &self,
        user_id: u64,
        limit: usize,
    ) -> TaskIterator<TaskResult<Vec<(User, Vec<String>)>>, SocialGraphOp> {
        // Based on mutual connections and interests
        let query = r#"
            MATCH (me:User {id: $user_id})-[:FOLLOWS]->(followed:User)
            MATCH (followed)-[:FOLLOWS]->(candidate:User)
            WHERE NOT (me)-[:FOLLOWS]->(candidate)
              AND candidate <> me

            // Also consider people with similar interests
            OPTIONAL MATCH (me)-[:INTERESTED_IN]->(interest:Topic)
            OPTIONAL MATCH (candidate)-[:INTERESTED_IN]->(interest)

            RETURN candidate,
                   collect(DISTINCT followed.name) AS mutual_connections,
                   collect(DISTINCT interest.name) AS shared_interests
            ORDER BY size(shared_interests) DESC, size(mutual_connections) DESC
            LIMIT $limit
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns (user, mutual_connections, shared_interests)
            Ok(vec![])
        }))
    }

    /// Detect community/influencers using centrality
    pub fn find_influencers(
        &self,
        topic: Option<String>,
        limit: usize,
    ) -> TaskIterator<TaskResult<Vec<(User, f32)>>, SocialGraphOp> {
        // PageRank-based influencer detection
        let query = if topic.is_some() {
            r#"
                MATCH (u:User)-[:INTERESTED_IN]->(t:Topic {name: $topic})
                WITH collect(u) AS community
                UNWIND community AS user
                MATCH (user)-[:FOLLOWS*]-(other:User)
                WHERE other IN community
                // Simplified PageRank approximation
                RETURN user, count(DISTINCT other) AS influence_score
                ORDER BY influence_score DESC
                LIMIT $limit
            "#
        } else {
            r#"
                // Global PageRank (via Graph Data Science library)
                CALL gds.pageRank.stream({
                    nodeProjection: 'User',
                    relationshipProjection: 'FOLLOWS',
                    maxIterations: 20,
                    dampingFactor: 0.85
                })
                YIELD nodeId, score
                MATCH (u:User) WHERE id(u) = nodeId
                RETURN u, score
                ORDER BY score DESC
                LIMIT $limit
            "#
        };

        TaskIterator::effect(Effect::Io(move || {
            // Returns (user, pagerank_score)
            Ok(vec![])
        }))
    }
}
```

## Part 2: Fraud Detection

### Transaction Fraud Graph

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from_account: u64,
    pub to_account: u64,
    pub amount: f64,
    pub timestamp: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum FraudPattern {
    RapidTransfer,      // Multiple transfers in short time
    CircularTransfer,   // Money returns to origin
    Structuring,        // Amounts just below reporting threshold
    UnusualPattern,     // Anomalous behavior
}

pub struct FraudDetectionService {
    neo4j_uri: String,
    username: String,
    password: String,
}

impl FraudDetectionService {
    pub fn detect_circular_transfers(
        &self,
        start_account: u64,
        max_depth: usize,
    ) -> TaskIterator<TaskResult<Vec<Vec<Transaction>>>, FraudPattern> {
        // Detect money laundering rings
        let query = r#"
            MATCH path = (start:Account {id: $start_account})-
                [:TRANSFERRED_TO*3..$max_depth]->(start)
            WHERE ALL(r IN relationships(path)
                WHERE r.timestamp < datetime() - duration('PT1H'))
            RETURN [r IN relationships(path) | r] AS transactions
            ORDER BY length(path) ASC
            LIMIT 10
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns cycles where money returns to origin
            Ok(vec![])
        }))
    }

    pub fn detect_structuring(
        &self,
        account_id: u64,
        time_window_hours: u64,
    ) -> TaskIterator<TaskResult<Vec<(u64, f64)>>, FraudPattern> {
        // Detect amounts just below $10,000 reporting threshold
        let query = r#"
            MATCH (account:Account {id: $account_id})
            MATCH (account)-[t:TRANSFERRED_TO]->(target)
            WHERE t.timestamp > datetime() - duration('PT' + $hours + 'H')
              AND t.amount >= 9000  // Near threshold
              AND t.amount < 10000  // Below threshold
            RETURN target.id AS target_account, sum(t.amount) AS total_amount
            GROUP BY target_account
            HAVING total_amount > 20000  // Multiple suspicious transfers
            ORDER BY total_amount DESC
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns (target_account, total_structured_amount)
            Ok(vec![])
        }))
    }

    pub fn detect_rapid_transfers(
        &self,
        account_id: u64,
        time_window_minutes: u64,
    ) -> TaskIterator<TaskResult<Vec<Transaction>>, FraudPattern> {
        // Detect rapid movement of funds
        let query = r#"
            MATCH (account:Account {id: $account_id})
            MATCH (account)-[t:TRANSFERRED_TO]->(target)
            WHERE t.timestamp > datetime() - duration('PT' + $minutes + 'M')
            WITH account, target, collect(t) AS transfers
            WHERE size(transfers) >= 3  // 3+ transfers in window
            UNWIND transfers AS t
            RETURN t
            ORDER BY t.timestamp DESC
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns suspicious rapid transfer transactions
            Ok(vec![])
        }))
    }

    pub fn find_fraud_rings(
        &self,
        min_ring_size: usize,
    ) -> TaskIterator<TaskResult<Vec<Vec<u64>>>, FraudPattern> {
        // Find connected components of suspicious accounts
        let query = r#"
            // Find accounts with suspicious activity
            MATCH (a:Account)
            WHERE a.flagged = true

            // Find connections between them
            MATCH (a)-[:TRANSFERRED_TO]-(connected:Account)
            WHERE connected.flagged = true

            // Use GDS to find connected components
            CALL gds.wcc.stream({
                nodeProjection: 'Account',
                relationshipProjection: {
                    TRANSFERRED: {
                        type: 'TRANSFERRED_TO',
                        orientation: 'UNDIRECTED'
                    }
                }
            })
            YIELD nodeId, componentId
            MATCH (a:Account) WHERE id(a) = nodeId
            RETURN componentId, collect(a.id) AS accounts
            HAVING size(accounts) >= $min_size
            ORDER BY size(accounts) DESC
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns groups of connected flagged accounts
            Ok(vec![])
        }))
    }
}
```

### Real-Time Fraud Scoring

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudScore {
    pub transaction_id: String,
    pub score: f32,      // 0.0 - 1.0
    pub factors: Vec<String>,
    pub recommendation: String,  // "approve", "review", "block"
}

impl FraudDetectionService {
    pub fn score_transaction(
        &self,
        transaction: &Transaction,
    ) -> TaskIterator<TaskResult<FraudScore>, FraudPattern> {
        // Real-time fraud scoring using graph features
        let query = r#"
            MATCH (from:Account {id: $from_account})
            MATCH (to:Account {id: $to_account})

            // Feature 1: Transaction velocity (from account)
            OPTIONAL MATCH (from)-[prev_tx:TRANSFERRED_TO]->()
            WHERE prev_tx.timestamp > datetime() - duration('PT1H')
            WITH from, to, count(prev_tx) AS from_velocity

            // Feature 2: Recipient velocity
            OPTIONAL MATCH ()-[to_tx:TRANSFERRED_TO]->(to)
            WHERE to_tx.timestamp > datetime() - duration('PT1H')
            WITH from, to, from_velocity, count(to_tx) AS to_velocity

            // Feature 3: First-time transaction?
            OPTIONAL MATCH (from)-[existing:TRANSFERRED_TO]->(to)
            WITH from, to, from_velocity, to_velocity,
                 CASE WHEN existing IS NULL THEN 1 ELSE 0 END AS first_time

            // Feature 4: Shortest path (already connected?)
            OPTIONAL MATCH path = shortestPath((from)-[:TRANSFERRED_TO*1..3]-(to))
            WITH from, to, from_velocity, to_velocity, first_time,
                 CASE WHEN path IS NULL THEN 0 ELSE length(path) END AS path_length

            // Calculate fraud score
            RETURN
                // High velocity = higher risk
                (CASE WHEN from_velocity > 5 THEN 0.2 ELSE 0 END) +
                (CASE WHEN to_velocity > 10 THEN 0.2 ELSE 0 END) +
                // First-time recipient = higher risk
                (first_time * 0.3) +
                // No prior connection = higher risk
                (CASE WHEN path_length = 0 THEN 0.3 ELSE 0 END)
                AS fraud_score
        "#;

        TaskIterator::effect(Effect::Io(move || {
            let score: f32 = 0.0; // From query result

            let recommendation = if score >= 0.7 {
                "block"
            } else if score >= 0.4 {
                "review"
            } else {
                "approve"
            };

            Ok(FraudScore {
                transaction_id: transaction.id.clone(),
                score,
                factors: vec![],
                recommendation: recommendation.to_string(),
            })
        }))
    }
}
```

## Part 3: Knowledge Graph

### Entity Relationship Graph

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Event,
    Concept,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub properties: std::collections::HashMap<String, String>,
}

pub struct KnowledgeGraphService {
    neo4j_uri: String,
    username: String,
    password: String,
}

impl KnowledgeGraphService {
    pub fn search_entities(
        &self,
        query_text: &str,
        entity_types: Option<Vec<EntityType>>,
    ) -> TaskIterator<TaskResult<Vec<Entity>>, ()> {
        // Full-text search across entities
        let cypher = r#"
            CALL db.index.fulltext.queryNodes(
                'entity_fulltext_index',
                $query_text + '*'
            )
            YIELD node AS entity, score

            // Optional type filter
            WITH entity, score
            WHERE $types IS NULL OR any(l IN labels(entity) WHERE l IN $types)

            RETURN entity, score
            ORDER BY score DESC
            LIMIT 20
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns matching entities with relevance scores
            Ok(vec![])
        }))
    }

    pub fn find_related_entities(
        &self,
        entity_id: &str,
        max_depth: usize,
        relationship_filter: Option<Vec<String>>,
    ) -> TaskIterator<TaskResult<Vec<(Entity, Vec<String>)>>, ()> {
        // Explore connected entities
        let cypher = if let Some(rel_types) = relationship_filter {
            format!(
                r#"
                MATCH (start:Entity {{id: $entity_id}})
                MATCH path = (start)-[r:{}*1..$max_depth]-(related:Entity)
                RETURN related, [r IN relationships(path) | type(r)] AS path_relations
                "#,
                rel_types.join("|")
            )
        } else {
            r#"
                MATCH (start:Entity {id: $entity_id})
                MATCH path = (start)-[*1..$max_depth]-(related:Entity)
                WHERE related <> start
                RETURN related, [r IN relationships(path) | type(r)] AS path_relations
                ORDER BY length(path) ASC
                LIMIT 50
            "#
        };

        TaskIterator::effect(Effect::Io(move || {
            // Returns (entity, relationship_path) pairs
            Ok(vec![])
        }))
    }

    pub fn find_common_connections(
        &self,
        entity1_id: &str,
        entity2_id: &str,
    ) -> TaskIterator<TaskResult<Vec<(Entity, Vec<String>, Vec<String>)>>, ()> {
        // Find how two entities are connected
        let cypher = r#"
            MATCH (e1:Entity {id: $entity1_id})
            MATCH (e2:Entity {id: $entity2_id})
            MATCH path = shortestPath((e1)-[*1..4]-(e2))
            WITH [n IN nodes(path) | n] AS entities,
                 [r IN relationships(path) | type(r)] AS relations
            UNWIND entities AS entity
            WITH entity, relations
            WHERE entity.id <> $entity1_id AND entity.id <> $entity2_id
            RETURN entity, relations
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns intermediate entities connecting the two
            Ok(vec![])
        }))
    }

    pub fn extract_subgraph(
        &self,
        seed_entity_ids: &[String],
        max_depth: usize,
    ) -> TaskIterator<TaskResult<(Vec<Entity>, Vec<Relationship>), ()> {
        // Extract relevant subgraph for visualization
        let cypher = r#"
            UNWIND $seed_ids AS seed_id
            MATCH (seed:Entity {id: seed_id})

            // Find connected subgraph
            MATCH path = (seed)-[*0..$max_depth]-(connected:Entity)

            // Collect nodes and relationships
            WITH
                collect(DISTINCT seed) + collect(DISTINCT connected) AS all_nodes,
                collect(DISTINCT r) AS all_rels
            WITH
                [n IN all_nodes | n] AS nodes,
                [rel IN all_rels | rel] AS relationships

            RETURN nodes, relationships
        "#;

        TaskIterator::effect(Effect::Io(move || {
            // Returns (entities, relationships) for visualization
            Ok((vec![], vec![]))
        }))
    }
}
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
