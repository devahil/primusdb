-- PrimusDB Vector Search Example
-- Vector search enables similarity queries using embeddings
--
-- Note: Vector search uses the vector engine. Tables must be
-- created with embedding columns for vector similarity search.

-- Create a table with vector support (conceptual)
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255),
    content TEXT,
    embedding FLOAT[]  -- Vector embedding for similarity search
);

-- Insert documents with embeddings
INSERT INTO documents (title, content, embedding)
VALUES ('Machine Learning', 'Introduction to ML concepts', ARRAY[0.1, 0.2, 0.3, 0.4]);

INSERT INTO documents (title, content, embedding)
VALUES ('Deep Learning', 'Advanced neural networks', ARRAY[0.15, 0.25, 0.35, 0.45]);

INSERT INTO documents (title, content, embedding)
VALUES ('Databases', 'Relational and NoSQL databases', ARRAY[0.9, 0.8, 0.7, 0.6]);

-- Standard queries (vector similarity search requires the vector index command)
SELECT id, title, content FROM documents WHERE title ILIKE '%learning%';
