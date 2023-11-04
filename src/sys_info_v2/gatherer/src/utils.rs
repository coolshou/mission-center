pub mod arraystring {
    pub trait ToArrayStringLossy {
        fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY>;
    }

    impl ToArrayStringLossy for str {
        fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
            let mut result = arrayvec::ArrayString::new();
            if self.len() > CAPACITY {
                for i in (0..CAPACITY).rev() {
                    if self.is_char_boundary(i) {
                        result.push_str(&self[0..i]);
                        break;
                    }
                }
            } else {
                result.push_str(self);
            }

            result
        }
    }

    impl ToArrayStringLossy for std::borrow::Cow<'_, str> {
        fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
            let mut result = arrayvec::ArrayString::new();
            if self.len() > CAPACITY {
                for i in (0..CAPACITY).rev() {
                    if self.is_char_boundary(i) {
                        result.push_str(&self[0..i]);
                        break;
                    }
                }
            } else {
                result.push_str(self);
            }

            result
        }
    }
}
