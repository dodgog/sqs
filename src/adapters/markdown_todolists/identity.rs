use std::collections::HashSet;

const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const ID_LEN: usize = 4;

/// Generate a random ID of 4 alphanumeric characters (e.g., "x3km").
/// Retries on collision with existing IDs.
pub fn generate_id(existing: &HashSet<String>) -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    loop {
        let id: String = (0..ID_LEN)
            .map(|_| ALPHABET[rng.random_range(0usize..ALPHABET.len())] as char)
            .collect();
        if !existing.contains(&id) {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_format() {
        let id = generate_id(&HashSet::new());
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn avoids_collisions() {
        let mut existing = HashSet::new();
        existing.insert("x3km".to_string());
        let id = generate_id(&existing);
        assert_ne!(id, "x3km");
        assert_eq!(id.len(), 4);
    }

    #[test]
    fn generates_unique_ids() {
        let existing = HashSet::new();
        let mut generated = HashSet::new();
        for _ in 0..100 {
            let id = generate_id(&existing);
            assert!(generated.insert(id.clone()), "duplicate id generated: {id}");
        }
    }
}
