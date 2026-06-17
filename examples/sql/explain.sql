-- PrimusDB EXPLAIN Example
-- Use EXPLAIN to understand query execution plans

-- Basic EXPLAIN
EXPLAIN SELECT * FROM users WHERE age > 25;

-- EXPLAIN with DML
EXPLAIN INSERT INTO users (name, email) VALUES ('Test', 'test@example.com');

-- EXPLAIN with JOIN
EXPLAIN SELECT u.name, o.amount
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE u.age > 21
ORDER BY o.amount DESC
LIMIT 10;
