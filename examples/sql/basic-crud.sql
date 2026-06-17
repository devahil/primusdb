-- PrimusDB Basic CRUD Example
-- Run: primusdb query -f examples/sql/basic-crud.sql

-- Create a table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE,
    age INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert records
INSERT INTO users (name, email, age) VALUES ('Alice', 'alice@example.com', 30);
INSERT INTO users (name, email, age) VALUES ('Bob', 'bob@example.com', 25);
INSERT INTO users (name, email, age) VALUES ('Charlie', 'charlie@example.com', 35);

-- Query records
SELECT * FROM users;
SELECT * FROM users WHERE age > 28 ORDER BY name;
SELECT COUNT(*) AS user_count, AVG(age) AS avg_age FROM users;

-- Update a record
UPDATE users SET age = 31 WHERE name = 'Alice';

-- Delete a record
DELETE FROM users WHERE name = 'Charlie';

-- Final state
SELECT * FROM users ORDER BY id;
