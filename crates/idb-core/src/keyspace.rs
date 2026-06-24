use crate::errors::{CoreError, CoreResult};

pub trait SpaceKeyMapper {
    fn encode(&self, coordinates: &[u32]) -> CoreResult<u128>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterleavedKeyMapper {
    pub bits_per_dimension: u8,
}

impl InterleavedKeyMapper {
    pub fn new(bits_per_dimension: u8) -> CoreResult<Self> {
        if bits_per_dimension == 0 || bits_per_dimension > 32 {
            return Err(CoreError::InvalidDimension(
                "bits_per_dimension must be in 1..=32".to_string(),
            ));
        }
        Ok(Self { bits_per_dimension })
    }
}

impl SpaceKeyMapper for InterleavedKeyMapper {
    fn encode(&self, coordinates: &[u32]) -> CoreResult<u128> {
        if coordinates.is_empty() {
            return Err(CoreError::InvalidDimension(
                "cannot encode empty coordinate vector".to_string(),
            ));
        }

        let total_bits = coordinates.len() as u32 * self.bits_per_dimension as u32;
        if total_bits > 128 {
            return Err(CoreError::InvalidDimension(format!(
                "encoded key needs {} bits, exceeds u128",
                total_bits
            )));
        }

        let mut key: u128 = 0;
        for bit in (0..self.bits_per_dimension).rev() {
            for coord in coordinates {
                let v = ((coord >> bit) & 1) as u128;
                key = (key << 1) | v;
            }
        }

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{InterleavedKeyMapper, SpaceKeyMapper};

    #[test]
    fn interleaving_key_is_deterministic() {
        let mapper = InterleavedKeyMapper::new(5).expect("valid mapper");
        let coords = vec![14, 12, 0, 28, 22, 8];

        let a = mapper.encode(&coords).expect("encode a");
        let b = mapper.encode(&coords).expect("encode b");

        assert_eq!(a, b);
    }

    proptest! {
        #[test]
        fn interleaving_property_is_stable(coords in proptest::collection::vec(0_u32..32_u32, 1..=8)) {
            let mapper = InterleavedKeyMapper::new(5).expect("valid mapper");
            let a = mapper.encode(&coords).expect("encode a");
            let b = mapper.encode(&coords).expect("encode b");
            prop_assert_eq!(a, b);
        }
    }
}
