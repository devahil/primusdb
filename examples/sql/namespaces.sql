-- PrimusDB Namespace SQL Example
-- Namespaces provide multi-tenancy and data isolation

-- Create tables in the default namespace
CREATE TABLE app_users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(100) UNIQUE NOT NULL,
    role VARCHAR(50) DEFAULT 'user'
);

CREATE TABLE app_sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES app_users(id),
    token VARCHAR(255) UNIQUE NOT NULL,
    expires_at TIMESTAMP
);

-- Insert sample data
INSERT INTO app_users (username, role) VALUES ('admin', 'admin');
INSERT INTO app_users (username, role) VALUES ('dev1', 'developer');

INSERT INTO app_sessions (user_id, token, expires_at)
VALUES (1, 'tok_' || gen_random_uuid(), CURRENT_TIMESTAMP + INTERVAL '24 hours');

-- Cross-table queries with joins
SELECT u.username, u.role, s.token, s.expires_at
FROM app_users u
LEFT JOIN app_sessions s ON u.id = s.user_id;
