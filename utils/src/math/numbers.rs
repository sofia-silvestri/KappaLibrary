pub fn get_primes_number( limit: u64) -> Vec<u64> {
    let mut primes = Vec::new();
    let mut counter = 3;
    primes.push(2); // 2 is the first prime number
    while counter <= limit {
        let mut is_prime = true;
        for val in &primes {
            if counter % val == 0 {
                is_prime = false;
                break;
            }
        }
        if is_prime {
            primes.push(counter);
        }
        counter += 2;
    }
    primes
}

pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let limit = (n as f64).sqrt() as u64 + 1;
    let prime_list = get_primes_number(limit);
    if prime_list.contains(&n) {
        return true;
    }
    false
}

pub fn factorize(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();
    let primes = get_primes_number((n as f64).sqrt() as u64 + 1);
    for prime in primes {
        while n % prime == 0 {
            factors.push(prime);
            n /= prime;
        }
    }
    factors
}

pub fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        return a;
    }
    gcd(b, a % b)
}

pub fn lcm(a: u64, b: u64) -> u64 {
    (a * b) / gcd(a, b)
}

pub fn fibonacci(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    let mut a = 0;
    let mut b = 1;
    let mut c = 0;
    for _ in 2..=n {
        c = a + b;
        a = b;
        b = c;
    }
    c
}

pub fn factorial(n: u64) -> u64 {
    if n == 0 {
        return 1;
    }
    let mut result = 1;
    for i in 1..=n {
        result *= i;
    }
    result
}

pub fn binomial_coefficient(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    factorial(n) / (factorial(k) * factorial(n - k))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_primes_number() {
        let primes = get_primes_number(30);
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
    }
}