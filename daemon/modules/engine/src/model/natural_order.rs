use std::{cmp::Ordering, iter::Peekable, str::Chars};

/// 名前の自然順。数字の並びは数値として、それ以外は大文字と小文字を区別せずに比べ、
/// それでも等しい名前はバイト列の順で決める。
pub struct NaturalOrder;

impl NaturalOrder {
    pub fn compare(a: &str, b: &str) -> Ordering {
        Self::compare_loosely(a, b).then_with(|| a.cmp(b))
    }

    fn compare_loosely(a: &str, b: &str) -> Ordering {
        let mut a = a.chars().peekable();
        let mut b = b.chars().peekable();

        loop {
            let ordering = match (a.peek().copied(), b.peek().copied()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => Self::compare_digits(&Self::take_digits(&mut a), &Self::take_digits(&mut b)),
                (Some(x), Some(y)) => {
                    a.next();
                    b.next();
                    x.to_lowercase().cmp(y.to_lowercase())
                }
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
    }

    fn take_digits(chars: &mut Peekable<Chars<'_>>) -> String {
        let mut digits = String::new();
        while let Some(c) = chars.next_if(char::is_ascii_digit) {
            digits.push(c);
        }
        digits
    }

    /// 桁数に上限を設けないよう、整数へ変換せず、先頭の 0 を除いた桁数と並びで比べる。
    fn compare_digits(a: &str, b: &str) -> Ordering {
        let a = a.trim_start_matches('0');
        let b = b.trim_start_matches('0');
        a.len().cmp(&b.len()).then_with(|| a.cmp(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(names: &[&'static str]) -> Vec<&'static str> {
        let mut names = names.to_vec();
        names.sort_by(|a, b| NaturalOrder::compare(a, b));
        names
    }

    #[test]
    fn compares_digits_as_numbers_test() {
        assert_eq!(NaturalOrder::compare("2", "10"), Ordering::Less);
        assert_eq!(sorted(&["10.jpg", "2.jpg", "1.jpg"]), ["1.jpg", "2.jpg", "10.jpg"]);
        assert_eq!(sorted(&["page10", "page9", "page100"]), ["page9", "page10", "page100"]);
    }

    #[test]
    fn ignores_case_before_byte_order_test() {
        assert_eq!(NaturalOrder::compare("a", "B"), Ordering::Less);
        assert_eq!(NaturalOrder::compare("A", "a"), Ordering::Less, "大文字と小文字を無視すると等しく、バイト列の順で決まる");
        assert_eq!(NaturalOrder::compare("a", "a"), Ordering::Equal);
    }

    #[test]
    fn breaks_ties_of_leading_zeros_by_bytes_test() {
        assert_eq!(NaturalOrder::compare("x01", "x1"), Ordering::Less, "数値としては等しく、バイト列の順で決まる");
        assert_eq!(NaturalOrder::compare("x1", "x01"), Ordering::Greater);
    }

    #[test]
    fn compares_digits_longer_than_u64_test() {
        assert_eq!(NaturalOrder::compare("x10000000000000000000000", "x99"), Ordering::Greater);
        assert_eq!(NaturalOrder::compare("x10000000000000000000000", "x10000000000000000000001"), Ordering::Less);
    }

    #[test]
    fn places_digits_before_letters_test() {
        assert_eq!(sorted(&["b", "1", "a", "10"]), ["1", "10", "a", "b"]);
    }
}
