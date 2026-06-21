# Issue 17: [Database] Missing Database Index on `transactions.created_at` Causes Slow Dashboard Queries

## Description

The `transactions` table defined in `DATABASE_SCHEMA.md` has indexes on `transaction_hash`, `from_wallet_id`, `to_wallet_id`, and `user_id`, but does not have a compound index on `(user_id, created_at)` — the most common query pattern for the user dashboard. When a user with thousands of transactions (e.g., a high-frequency security researcher) loads their transaction history, the database performs a sequential scan filtered by `user_id` then sorts by `created_at`, which degrades as transaction volume grows. The `queries.rs` module in `src/database/queries.rs` implements `get_user_transactions_paginated()` which queries `WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3` — this query currently performs a full sort on every paginated page load. Users with 50,000+ transactions experience dashboard load times exceeding 8 seconds.

## Acceptance Criteria

- [ ] Create a new migration (`008_add_transaction_user_time_index.sql`) that adds a B-tree compound index on `transactions(user_id, created_at DESC)`
- [ ] Also add an index on `transactions.status` for filtering by transaction status (pending, confirmed, failed)
- [ ] Verify the index is used by running `EXPLAIN ANALYZE` on the paginated query with 100,000 test rows
- [ ] Measure and document the query performance improvement (target: sub-100ms for paginated queries with 100k+ rows)
- [ ] Update `src/database/queries.rs` to add index hints or query hints if needed for the PostgreSQL query planner
- [ ] Write a database benchmark test in `src/database/tests.rs` that measures query latency before and after the index

## Additional Context

Key files: `DATABASE_SCHEMA.md`, `src/database/queries.rs`, `src/database/models.rs`, `migrations/`.
