//! Merkle tree construction over integrity records.
//!
//! A checkpoint anchors a batch of records under a single root digest using a
//! binary Merkle tree over the canonical record hashes. Odd nodes are
//! self-paired (hashed with themselves) so any tree size produces a root.

use sha2::{Digest, Sha256};

/// Builds the Merkle root over a set of leaf hashes.
///
/// Leaves are hashed pairwise; when the level has an odd number of nodes the
/// final node is paired with itself. An empty set produces the hash of an
/// empty string so callers never have to special-case "no root".
pub fn merkle_root(leaf_hashes: &[String]) -> String {
    if leaf_hashes.is_empty() {
        return hex::encode(Sha256::digest(b""));
    }
    let mut level: Vec<Vec<u8>> = leaf_hashes
        .iter()
        .map(|s| hex::decode(s).unwrap_or_default())
        .collect();
    if level.len() == 1 {
        // A lone leaf is self-paired so every tree yields a non-trivial root.
        return hex::encode(hash_pair(&level[0], &level[0]));
    }
    while level.len() > 1 {
        let mut next: Vec<Vec<u8>> = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                left // self-pair odd node
            };
            next.push(hash_pair(left, right));
            i += 2;
        }
        level = next;
    }
    hex::encode(&level[0])
}

fn hash_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().to_vec()
}

/// SHA-256 of the concatenation of every leaf hash. Used for diagnostics and
/// cheap chain comparisons during reconciliation.
pub fn linear_accumulator(leaf_hashes: &[String]) -> String {
    let mut hasher = Sha256::new();
    for h in leaf_hashes {
        hasher.update(h.as_bytes());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_known_value() {
        // Two equal leaves hash(leaf||leaf).
        let a = hex::encode(Sha256::digest(b"a"));
        let root = merkle_root(&[a.clone(), a.clone()]);
        let mut h = Sha256::new();
        h.update(hex::decode(&a).unwrap());
        h.update(hex::decode(&a).unwrap());
        assert_eq!(root, hex::encode(h.finalize()));
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, hex::encode(Sha256::digest(b"")));
    }

    #[test]
    fn test_merkle_root_single_leaf_self_pairs() {
        let a = hex::encode(Sha256::digest(b"x"));
        let root = merkle_root(&[a.clone()]);
        let mut h = Sha256::new();
        h.update(hex::decode(&a).unwrap());
        h.update(hex::decode(&a).unwrap());
        assert_eq!(root, hex::encode(h.finalize()));
    }

    #[test]
    fn test_merkle_root_three_leaves() {
        let leaves: Vec<String> = (0..3).map(|i| hex::encode(Sha256::digest([i]))).collect();
        let root = merkle_root(&leaves);
        let mut h = Sha256::new();
        h.update(hex::decode(&leaves[0]).unwrap());
        h.update(hex::decode(&leaves[1]).unwrap());
        let h01 = h.finalize();
        let mut h22 = Sha256::new();
        h22.update(hex::decode(&leaves[2]).unwrap());
        h22.update(hex::decode(&leaves[2]).unwrap());
        let h22 = h22.finalize();
        let mut h2 = Sha256::new();
        h2.update(h01);
        h2.update(h22);
        assert_eq!(root, hex::encode(h2.finalize()));
    }

    #[test]
    fn test_linear_accumulator_changes_with_order() {
        let a = hex::encode(Sha256::digest(b"a"));
        let b = hex::encode(Sha256::digest(b"b"));
        assert_ne!(
            linear_accumulator(&[a.clone(), b.clone()]),
            linear_accumulator(&[b, a])
        );
    }
}
