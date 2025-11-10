use crate::math::complex::Complex;

#[derive(Debug, Clone, PartialEq)]
pub struct ComplexVector<T> {
    pub real: Vec<T>,
    pub imag: Vec<T>,
}

impl<T: Clone + num_traits::Zero> ComplexVector<T> {
    pub fn new(real: Vec<T>, imag: Option<Vec<T>>) -> Self {
        if let Some(imag) = imag {
            Self { real, imag }
        } else {
            let imag = vec![T::zero(); real.len()];
            Self { real, imag }
        }
    }
    pub fn from_complex_numbers(numbers: Vec<Complex<T>>) -> Self {
        let real = numbers.iter().map(|c| c.real.clone()).collect();
        let imag = numbers.iter().map(|c| c.imag.clone()).collect();
        Self { real, imag }
    }

    pub fn to_complex_numbers(&self) -> Vec<Complex<T>>
    {
        self.real
            .iter()
            .cloned()
            .zip(self.imag.iter().cloned())
            .map(|(r, i)| Complex { real: r, imag: i })
            .collect()
    }

    pub fn from_mag_phase(magnitudes: Vec<T>, phases: Vec<T>) -> Self
    where
        T: Copy + num_traits::Float,
    {
        let real: Vec<T> = magnitudes
            .iter()
            .cloned()
            .zip(phases.iter().cloned())
            .map(|(mag, phase)| mag * phase.cos())
            .collect();
        let imag: Vec<T> = magnitudes
            .iter()
            .cloned()
            .zip(phases.iter().cloned())
            .map(|(mag, phase)| mag * phase.sin())
            .collect();
        Self { real, imag }
    }

    pub fn to_mag_phase(&self) -> (Vec<T>, Vec<T>)
    where
        T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Float,
    {
        let magnitudes: Vec<T> = self
            .real
            .iter()
            .cloned()
            .zip(self.imag.iter().cloned())
            .map(|(real, imag)| (real * real + imag * imag).sqrt())
            .collect();
        let phases: Vec<T> = self
            .real
            .iter()
            .cloned()
            .zip(self.imag.iter().cloned())
            .map(|(real, imag)| imag.atan2(real))
            .collect();
        (magnitudes, phases)
    }

    pub fn zeroed(size: usize) -> Self {
        Self {
            real: vec![T::zero(); size],
            imag: vec![T::zero(); size],
        }
    }
    pub fn len(&self) -> usize {
        self.real.len()
    }
    pub fn is_empty(&self) -> bool {
        self.real.is_empty()
    }
    pub fn get(&self, index: usize) -> Option<Complex<T>> {
        if index < self.len() {
            Some(Complex {
                real: self.real[index].clone(),
                imag: self.imag[index].clone(),
            })
        } else {
            None
        }
    }

    pub fn push(&mut self, number: Complex<T>) {
        self.real.push(number.real);
        self.imag.push(number.imag);
    }

    pub fn iter(&self) -> impl Iterator<Item = Complex<T>> + '_ {
        self.real
            .iter()
            .cloned()
            .zip(self.imag.iter().cloned())
            .map(|(r, i)| Complex { real: r, imag: i })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = Complex<&mut T>> + '_
    where
        T: Clone,
    {
        self.real
            .iter_mut()
            .zip(self.imag.iter_mut())
            .map(|(r, i)| Complex { real: r, imag: i })
    }

    pub fn clear(&mut self) {
        self.real.clear();
        self.imag.clear();
    }

    pub fn resize(&mut self, new_size: usize, value: Complex<T>) {
        self.real.resize(new_size, value.real);
        self.imag.resize(new_size, value.imag);
    }

    pub fn extend_from_slice(&mut self, other: &[Complex<T>]) {
        for number in other {
            self.push(number.clone());
        }
    }

    pub fn as_slices(&self) -> (&[T], &[T]) {
        (&self.real, &self.imag)
    }
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        (&mut self.real, &mut self.imag)
    }

    pub fn clone_from_slice(&mut self, other: &[Complex<T>]) {
        self.clear();
        self.extend_from_slice(other);
    }
    pub fn from_slice(other: &[Complex<T>]) -> Self {
        let mut complex_vector = ComplexVector {
            real: Vec::new(),
            imag: Vec::new(),
        };
        complex_vector.extend_from_slice(other);
        complex_vector
    }

    pub fn to_owned(&self) -> Self {
        Self {
            real: self.real.clone(),
            imag: self.imag.clone(),
        }
    }

    pub fn abs(&self) -> Vec<T>
    where
        T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Float,
    {
        self.iter()
            .map(|c| {
                let real: T = c.real;
                let imag: T = c.imag;
                (real * real + imag * imag).sqrt()
            })
            .collect()
    }

    pub fn phase(&self) -> Vec<T>
    where
        T: Copy + num_traits::Float,
    {
        self.iter()
            .map(|c| {
                let real: T = c.real;
                let imag: T = c.imag;
                imag.atan2(real)
            })
            .collect()
    }

    pub fn conjugate_inplace(&mut self)
    where
        T: std::ops::Neg<Output = T> + Clone,
    {
        for i in &mut self.imag {
            *i = -i.clone();
        }
    }   
    pub fn conjugate(&self) -> Self
    where
        T: Clone + std::ops::Neg<Output = T>,
    {
        let imag_conjugated: Vec<T> = self.imag.iter().cloned().map(|i| -i).collect();
        Self {
            real: self.real.clone(),
            imag: imag_conjugated,
        }
    }

    pub fn sqrt(&self) -> Self
    where
        T: Copy + std::ops::Mul<Output = T> + std::ops::Sub<Output = T> + std::ops::Add<Output = T> + num_traits::Float,
    {
        let mut result_real: Vec<T> = Vec::with_capacity(self.len());
        let mut result_imag: Vec<T> = Vec::with_capacity(self.len());
        for c in self.iter() {
            let real = c.real;
            let imag = c.imag;
            let magnitude = (real * real + imag * imag).sqrt();
            let phase = imag.atan2(real) / T::from(2.0).unwrap();
            result_real.push(magnitude * phase.cos());
            result_imag.push(magnitude * phase.sin());
        }
        Self {
            real: result_real,
            imag: result_imag,
        }
    }

    pub fn scale_inplace(&mut self, factor: T)
    where
        T: Clone + std::ops::Mul<Output = T>,
    {
        for r in &mut self.real {
            *r = r.clone() * factor.clone();
        }
        for i in &mut self.imag {
            *i = i.clone() * factor.clone();
        }
    }

    pub fn scale(&self, factor: T) -> Self
    where
        T: Clone + std::ops::Mul<Output = T>,
    {
        let real: Vec<T> = self.real.iter().cloned().map(|r| r * factor.clone()).collect();
        let imag: Vec<T> = self.imag.iter().cloned().map(|i| i * factor.clone()).collect();
        Self { real, imag }
    }

    pub fn norm(&self) -> T
    where
        T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Float,
    {
        let mut sum = T::zero();
        for c in self.iter() {
            let real = c.real;
            let imag = c.imag;
            sum = sum + real * real + imag * imag;
        }
        sum.sqrt()
    }
}

impl<T> std::ops::Add for ComplexVector<T>
    where T : std::ops::Add<Output = T> + Clone
{
    type Output = ComplexVector<T>;

    fn add(self, other: ComplexVector<T>) -> ComplexVector<T> {
        let real: Vec<T> = self.real.iter().cloned().zip(other.real.iter().cloned())
            .map(|(a, b)| a + b).collect();
        let imag: Vec<T> = self.imag.iter().cloned().zip(other.imag.iter().cloned())
            .map(|(a, b)| a + b).collect();
        ComplexVector { real, imag }
    }
}

impl<T> std::ops::Neg for ComplexVector<T> 
    where T : std::ops::Neg<Output = T> + Clone
{
    type Output = ComplexVector<T>;

    fn neg(self) -> ComplexVector<T> {
        let real: Vec<T> = self.real.iter().cloned()
            .map(|a| -a).collect();
        let imag: Vec<T> = self.imag.iter().cloned()
            .map(|a| -a).collect();
        ComplexVector { real, imag }
    }
    
}

impl<T> std::ops::Sub for ComplexVector<T> 
    where T : std::ops::Sub<Output = T> 
    + std::ops::Neg<Output = T> 
    + std::ops::Add<Output = T>
    + Clone
{
    type Output = ComplexVector<T>;

    fn sub(self, other: ComplexVector<T>) -> ComplexVector<T> {
        self + (-other)
    }
}

impl<T> std::ops::Mul for ComplexVector<T> 
    where T : std::ops::Mul<Output = T> 
    + std::ops::Sub<Output = T> 
    + std::ops::Add<Output = T>
    + Clone
{
    type Output = ComplexVector<T>;

    fn mul(self, other: ComplexVector<T>) -> ComplexVector<T> {
        let real: Vec<T> = self.real.iter().cloned().zip(other.real.iter().cloned())
            .zip(self.imag.iter().cloned().zip(other.imag.iter().cloned()))
            .map(|((a_real, b_real), (a_imag, b_imag))| {
                a_real.clone() * b_real.clone() - a_imag.clone() * b_imag.clone()
            }).collect();
        let imag: Vec<T> = self.real.iter().cloned().zip(other.imag.iter().cloned())
            .zip(self.imag.iter().cloned().zip(other.real.iter().cloned()))
            .map(|((a_real, b_imag), (a_imag, b_real))| {
                a_real.clone() * b_imag.clone() + a_imag.clone() * b_real.clone()
            }).collect();
        ComplexVector { real, imag }
    }
}

