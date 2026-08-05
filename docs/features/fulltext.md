# Full-Text Search

> **NOT AVAILABLE**: The full-text engine described here lives in `src/fulltext.rs`,
> which is not declared in `src/lib.rs` and is **not compiled** in the current
> build. This page is kept as reference for the intended design.

PrimusDB includes a built-in full-text search engine that provides
term-based search with TF-IDF relevance scoring. The engine is
implemented as a self-contained inverted index (`src/fulltext.rs`)
and is available as an index type in the **Document storage engine**.

---

## Overview

The full-text search engine tokenises text fields, builds an inverted
index mapping terms to document IDs, and supports boolean queries
with ranked results. It is designed for use cases such as:

- Searching product descriptions in an e‑commerce catalogue
- Filtering support tickets by keywords
- Implementing document-level search within JSON collections
- Powering custom search endpoints in applications

The engine is **collection-scoped**: each `FullTextIndex` instance
is independent and operates on a single logical collection of
documents.

---

## Architecture

### Inverted Index

The core data structure is an **inverted index**, a hash map from
each unique term to the set of document IDs that contain it:

```
inverted_index: HashMap<String, HashSet<u64>>
```

Alongside the inverted index, the engine maintains a **document
term-frequency table** that records how many times each term appears
in each document:

```
doc_frequencies: HashMap<u64, HashMap<String, usize>>
```

This two-table design allows the engine to answer both "which
documents contain this term?" (via the inverted index) and "how
relevant is this term to a document?" (via the term-frequency
table).

### Tokenisation

When a document is indexed, its text is tokenised in a single pass:

1. Characters are lowercased.
2. Only alphanumeric characters and apostrophes (`'`) are kept;
   everything else is treated as a token separator.
3. Empty tokens are discarded.
4. If **stop words** are configured, matching tokens are filtered
   out.

Example:

```
Input:  "Hello, World! It's nice."
Output: ["hello", "world", "it's", "nice"]
```

### TF-IDF Scoring

Search results are returned as `(doc_id, score)` pairs sorted by
descending score. The score is the sum of per-term **TF-IDF**
values:

- **TF (Term Frequency)** — `log(1 + count)`, where `count` is the
  number of times the term appears in the document. The log
  dampens the contribution of very frequent terms within a single
  document.

- **IDF (Inverse Document Frequency)** — `log(N / df)`, where `N`
  is the total number of documents in the index and `df` is the
  number of documents that contain the term. This penalises terms
  that appear in many documents (e.g. stop words that were not
  filtered).

The final score for a document is the sum of TF-IDF across all
query terms:

```
score(doc) = Σ  TF(term, doc) × IDF(term)
        term ∈ query
```

---

## Supported Search Modes

### AND Mode

**All** query terms must appear in the document. The candidate set
is the **intersection** of the postings lists for each term.

| Behaviour               | Detail                                     |
|-------------------------|--------------------------------------------|
| Matching                | Documents containing every term            |
| Ranking                 | TF-IDF sum over all query terms            |
| Use case                | Precision-oriented search                  |

```
search("quick fox", SearchMode::And)
// Returns docs that contain BOTH "quick" AND "fox"
```

### OR Mode

**Any** query term may appear in the document. The candidate set is
the **union** of the postings lists for each term.

| Behaviour               | Detail                                     |
|-------------------------|--------------------------------------------|
| Matching                | Documents containing at least one term     |
| Ranking                 | TF-IDF sum over present terms              |
| Use case                | Recall-oriented search / fuzzy matching    |

```
search("fast sleeps", SearchMode::Or)
// Returns docs that contain EITHER "fast" OR "sleeps" (or both)
```

### Phrase Mode

All query terms must appear in the document (same as AND), but the
mode signals an intent for exact-phrase semantics. The current
implementation requires all terms to be present; future versions
may enforce term ordering and proximity.

| Behaviour               | Detail                                     |
|-------------------------|--------------------------------------------|
| Matching                | Documents containing all phrase terms      |
| Ranking                 | TF-IDF sum over all query terms            |
| Use case                | Quoted-string search                       |

```
search("lazy dog", SearchMode::Phrase)
// Returns docs that contain BOTH "lazy" AND "dog"
```

---

## Usage Examples

### Rust API (library)

```rust
use primusdb::fulltext::{FullTextIndex, SearchMode};

let mut idx = FullTextIndex::new();

// Index documents
idx.index_document(1, "The quick brown fox jumps over the lazy dog");
idx.index_document(2, "A quick brown fox is fast");
idx.index_document(3, "The lazy dog sleeps all day");

// AND search — documents must contain all terms
let results = idx.search("quick fox", SearchMode::And);
// Returns [(1, score), (2, score)]

// OR search — documents may contain any term
let results = idx.search("fast sleeps", SearchMode::Or);
// Returns [(2, score), (3, score)]

// Phrase search
let results = idx.search("lazy dog", SearchMode::Phrase);
// Returns [(1, score), (3, score)]

// Remove a document
idx.remove_document(2);

// Inspect index statistics
println!("Total documents: {}", idx.len());
println!("Unique terms: {}", idx.unique_term_count());
```

### Re-indexing

Re-indexing a document ID replaces its previous terms entirely:

```rust
idx.index_document(1, "hello world");
idx.index_document(1, "foo bar");        // replaces "hello world"

let results = idx.search("hello", SearchMode::And);
// Empty — doc 1 no longer contains "hello"
```

### With Stop Words

```rust
use std::collections::HashSet;

let stop: HashSet<String> =
    ["the", "a", "an", "is", "and"]
        .iter().map(|s| s.to_string()).collect();

let mut idx = FullTextIndex::with_stop_words(stop);
idx.index_document(1, "The cat and a dog");

// Tokens indexed: ["cat", "dog"]
// Stop words "the", "and", "a" are excluded
```

### Document Storage Engine Integration

In the Document engine, `FullText` is a first-class index type:

```rust
pub enum DocumentIndexType {
    BTree,
    Hash,
    FullText,   // <-- full-text index
    GeoSpatial,
}
```

When a field is indexed with `FullText`, the engine creates a
`FullTextIndex` for that field and automatically updates it on
insert, update, and delete operations.

---

## Stop Words Configuration

Stop words are common words that are filtered out during
tokenisation so they do not occupy space in the inverted index or
affect search results. Typical stop words include articles,
prepositions, and very common verbs.

### Default Behaviour

By default (`FullTextIndex::new()`), **no stop words are applied**.
All tokens are indexed.

### Custom Stop Words

You can provide a custom set of stop words via the constructor:

```rust
let stop: HashSet<String> = [
    "the", "a", "an", "and", "or", "but",
    "is", "are", "was", "were", "be", "been",
    "in", "on", "at", "to", "for", "of", "by",
    "with", "from", "that", "this", "it",
].iter().map(|s| s.to_string()).collect();

let mut idx = FullTextIndex::with_stop_words(stop);
```

### How Stop Words Are Applied

During tokenisation, every candidate token is checked against the
stop-word set. If a match is found, the token is discarded and does
**not** appear in:

- The inverted index
- The document term-frequency table
- Any query result

This means searching for a stop word (e.g. `"the"`) when stop words
are active will return zero results.

### Language-Specific Stems

The engine does not include built-in language-specific stop-word
lists. Applications should supply their own set appropriate to
their data language. An English stop-word list is shown above; for
other languages, standard stop-word lists can be used.

---

## Performance Considerations

### Index Size

The inverted index grows linearly with the number of unique terms.
Each term stores a `HashSet<u64>` of document IDs, so:

- **Many short documents** → a relatively small number of terms per
  document; the index stays compact.
- **Few long documents** → many unique terms; the index grows
  proportionally to vocabulary size.
- **Stop words** reduce index size by excluding high-frequency
  terms that appear in almost every document.

### Memory

The `FullTextIndex` struct lives entirely in memory as
`HashMap`-backed data structures. For large collections (millions
of documents), memory usage may become significant. Consider:

- Using stop words to reduce the term vocabulary.
- Sharding collections so each index remains small.
- Persisting the index via serialisation (`serde` derives are
  implemented) and restoring on startup.

### Query Performance

- **AND mode** is typically fastest because the intersection of
  postings lists reduces the candidate set early.
- **OR mode** can be slower on broad queries because the union may
  include most documents in the collection.
- **Phrase mode** currently has the same performance profile as AND;
  future proximity checks may add overhead.

### Tokenisation Overhead

Tokenisation is a simple character-by-character scan with no
external dependencies. It is fast, but for very large text fields
(>100 KB per document), the cumulative cost of tokenisation should
be factored into indexing throughput.

### Re-indexing

Calling `index_document` on an already-indexed document ID performs
a full remove-then-insert cycle. This is safe and correct but
involves scanning and deleting all previous term associations
before adding the new ones. For bulk updates, consider batching
operations.

### Thread Safety

`FullTextIndex` is **not** thread-safe by itself (no `Sync` / `Send`
wrappers). In a multi-threaded server, each index should be
protected by a `Mutex` or `RwLock`, or confined to a single
thread. The document engine uses `Arc<RwLock<...>>` to protect its
collection-level indexes.

---

## Data Structures

### `FullTextIndex`

| Field              | Type                                      | Description                           |
|--------------------|-------------------------------------------|---------------------------------------|
| `inverted_index`   | `HashMap<String, HashSet<u64>>`           | Term → set of document IDs            |
| `doc_frequencies`  | `HashMap<u64, HashMap<String, usize>>`    | Doc ID → (term → count)               |
| `total_docs`       | `u64`                                     | Number of indexed documents           |
| `stop_words`       | `Option<HashSet<String>>`                 | Optional stop-word filter             |

### `SearchMode`

| Variant   | Semantics              |
|-----------|------------------------|
| `And`     | All terms must match   |
| `Or`      | Any term must match    |
| `Phrase`  | Exact phrase match     |

### Key Methods

| Method                        | Description                               |
|-------------------------------|-------------------------------------------|
| `FullTextIndex::new()`        | Creates an empty index (no stop words)    |
| `with_stop_words(stop_words)` | Creates an index with a stop-word set     |
| `index_document(doc_id, text)`| Indexes (or replaces) a document          |
| `remove_document(doc_id)`     | Removes a document from the index         |
| `search(query, mode)`         | Searches and returns ranked results       |
| `len()`                       | Returns the number of indexed documents   |
| `is_empty()`                  | Returns `true` if the index is empty      |
| `unique_term_count()`         | Returns the number of unique terms        |

---

## Limitations and Future Work

- **No proximity scoring** — Phrase mode does not yet enforce term
  ordering or positional distance.
- **No stemming / lemmatisation** — Terms are indexed as-is;
  "running" and "run" are separate terms.
- **No fuzzy / prefix matching** — Wildcards, prefixes, and edit
  distance are not supported.
- **Single-field scope** — Each index operates on a single text
  field; cross-field search requires application-level merging.
- **No SQL `MATCH ... AGAINST` syntax** — Full-text search is
  exposed via the Rust API and the Document engine's index types.
  SQL-level integration is planned for a future release.
