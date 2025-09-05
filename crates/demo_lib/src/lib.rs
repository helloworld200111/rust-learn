// This is the demo_lib crate, providing calculation and addition functions.

pub fn calculate(expr: &str) -> Result<f64, meval::Error> {
    meval::eval_str(expr)
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_calculate() {
        assert_eq!(calculate("(1+1)*2").unwrap(), 4.0);
        assert_eq!(calculate("10 / 2 - 3").unwrap(), 2.0);
    }
}