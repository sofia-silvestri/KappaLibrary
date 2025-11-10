use num_traits::{Float, Zero, One};


#[derive(Debug, PartialEq)]
pub struct Complex<T> {
    pub real: T,
    pub imag: T,
}

impl<T> Complex<T> 
where T: 
      Float 
    + Clone  
    + std::fmt::Debug
    + std::fmt::Display
    {
    pub fn new(real: T, imag: T) -> Self {
        Complex { real, imag }
    }
    pub fn real(&self) -> T {
        self.real.clone()
    }
    pub fn imag(&self) -> T {
        self.imag.clone()
    }
    pub fn magnitude(&self) -> T
    where
        T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Float,
    {
        (self.real * self.real + self.imag * self.imag).sqrt()
    }
    pub fn phase(&self) -> T
    where
        T: Copy + num_traits::Float,
    {
        self.imag.atan2(self.real)
    }
    pub fn conjugate(&self) -> Self
    where
        T: std::ops::Neg<Output = T> + Clone,
    {
        Complex {
            real: self.real.clone(),
            imag: -self.imag.clone(),
        }
    }
    pub fn i() -> Self
    where
        T: num_traits::Zero + num_traits::One,
    {
        Complex {
            real: T::zero(),
            imag: T::one(),
        }
    }

    pub fn from_polar(magnitude: T, phase: T) -> Self {
        Complex::new(magnitude * phase.cos(), magnitude * phase.sin())
    }

    pub fn to_polar(&self) -> (T, T) {
        (self.magnitude(), self.phase())
    }

    pub fn abs(&self) -> T {
        self.magnitude()
    }

    pub fn sqrt(&self) -> Self {
        let r = self.magnitude().sqrt();
        let theta = self.phase() / T::from(2.0).unwrap();
        Complex::from_polar(r, theta)
    }

    pub fn powi(&self, exponent: i32) -> Self {
        let r = self.magnitude().powi(exponent);
        let theta = self.phase() * T::from(exponent).unwrap();
        Complex::from_polar(r, theta)
    }

    pub fn powf(&self, exponent: T) -> Self {
        let r = self.magnitude().powf(exponent);
        let theta = self.phase() * exponent;
        Complex::from_polar(r, theta)
    }

    pub fn exp(&self) -> Self {
        let exp_real = self.real.exp();
        Complex::new(exp_real * self.imag.cos(), exp_real * self.imag.sin())
    }

    pub fn ln(&self) -> Self {
        let r = self.magnitude().ln();
        let theta = self.phase();
        Complex::new(r, theta)
    }

    pub fn log(&self, base: T) -> Self {
        let r = self.magnitude().log(base);
        let theta = self.phase();
        Complex::new(r, theta)
    }

    pub fn sin(&self) -> Self {
        Complex::new(self.real.sin() * self.imag.cosh(), self.real.cos() * self.imag.sinh())
    }

    pub fn asin(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.sin() / r, theta.cos() / r)
    }

    pub fn cos(&self) -> Self {
        Complex::new(self.real.cos() * self.imag.cosh(), -self.real.sin() * self.imag.sinh())
    }

    pub fn acos(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.cos() / r, -theta.sin() / r)
    }

    pub fn tan(&self) -> Self {
        let denom = self.cos();
        if denom.real == T::zero() && denom.imag == T::zero() {
            panic!("Division by zero in tan");
        }
        self.sin() / denom
    }

    pub fn atan(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.sin() / r, theta.cos() / r)
    }

    pub fn atan2(y: T, x: T) -> Self {
        let r = (x * x + y * y).sqrt();
        let theta = y.atan2(x);
        Complex::new(r, theta)
    }

    pub fn sinh(&self) -> Self {
        Complex::new(self.real.sinh() * self.imag.cos(), self.real.cosh() * self.imag.sin())
    }

    pub fn asinh(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.sinh() / r, theta.cosh() / r)
    }

    pub fn cosh(&self) -> Self {
        Complex::new(self.real.cosh() * self.imag.cos(), self.real.sinh() * self.imag.sin())
    }

    pub fn acosh(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.cosh() / r, theta.sinh() / r)
    }

    pub fn tanh(&self) -> Self {
        let denom = self.cosh();
        if denom.real == T::zero() && denom.imag == T::zero() {
            panic!("Division by zero in tanh");
        }
        self.sinh() / denom
    }

    pub fn atanh(&self) -> Self {
        let r = self.magnitude();
        let theta = self.phase();
        Complex::new(theta.sinh() / r, theta.cosh() / r)
    }
    
}

impl<T: Clone> Clone for Complex<T> {
    fn clone(&self) -> Self {
        Complex {
            real: self.real.clone(),
            imag: self.imag.clone(),
        }
    }
}
impl <T> Zero for Complex<T> 
where T : num_traits::Zero 
        + Float 
        + std::fmt::Display 
        + std::fmt::Debug
        + Clone
{
    fn zero() -> Self {
        Complex::new(T::zero(), T::zero())
    }

    fn is_zero(&self) -> bool {
        self.real == T::zero() && self.imag == T::zero()
    }
}

impl <T> One for Complex<T> 
where T : num_traits::Zero 
        + Float 
        + std::fmt::Display 
        + std::fmt::Debug
        + Clone
{
    fn one() -> Self {
        Complex::new(T::one(), T::zero())
    }

    fn is_one(&self) -> bool {
        self.real == T::one() && self.imag == T::zero()
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::Add for Complex<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Complex::new(self.real + other.real, self.imag + other.imag)
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::AddAssign for Complex<T> {
    fn add_assign(&mut self, other: Self) {
        self.real = self.real + other.real;
        self.imag = self.imag + other.imag;
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::Sub for Complex<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Complex::new(self.real - other.real, self.imag - other.imag)
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::SubAssign for Complex<T> {
    fn sub_assign(&mut self, other: Self) {
        self.real = self.real - other.real;
        self.imag = self.imag - other.imag;
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::Mul for Complex<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let real = self.real * other.real - self.imag * other.imag;
        let imag = self.real * other.imag + self.imag * other.real;
        Complex::new(real, imag)
    }

}
impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::MulAssign for Complex<T> {
    fn mul_assign(&mut self, other: Self) {
        let real = self.real * other.real - self.imag * other.imag;
        let imag = self.real * other.imag + self.imag * other.real;
        self.real = real;
        self.imag = imag;
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::Div for Complex<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let denom = other.real * other.real + other.imag * other.imag;
        let real = (self.real * other.real + self.imag * other.imag) / denom;
        let imag = (self.imag * other.real - self.real * other.imag) / denom;
        Complex::new(real, imag)
    }

}
impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::DivAssign for Complex<T> {
    fn div_assign(&mut self, other: Self) {
        let denom = other.real * other.real + other.imag * other.imag;
        let real = (self.real * other.real + self.imag * other.imag) / denom;
        let imag = (self.imag * other.real - self.real * other.imag) / denom;
        self.real = real;
        self.imag = imag;
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> std::ops::Neg for Complex<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Complex::new(-self.real, -self.imag)
    }
}

impl<T: Float + std::fmt::Display + std::fmt::Debug> From<T> for Complex<T> {
    fn from(value: T) -> Self {
        Complex::new(value, T::zero())
    }
}

impl<T: Copy> Copy for Complex<T> { }

