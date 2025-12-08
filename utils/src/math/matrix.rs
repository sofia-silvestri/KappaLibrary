use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Matrix<T> {
    pub data: Vec<Vec<T>>,
    pub rows: usize,
    pub cols: usize,
}

impl<T> Matrix<T>
where
    T: Clone + Default,
{
    pub fn new(rows: usize, cols: usize) -> Self {
        let data = vec![vec![T::default(); cols]; rows];
        Matrix { data, rows, cols }
    }

    pub fn from_vec(data: Vec<Vec<T>>) -> Self {
        let rows = data.len();
        let cols = if rows > 0 { data[0].len() } else { 0 };
        Matrix { data, rows, cols }
    }

    pub fn to_vec(&self) -> Vec<Vec<T>> {
        self.data.clone()
    }
    
    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row < self.rows && col < self.cols {
            Some(&self.data[row][col])
        } else {
            None
        }
    }

    pub fn set(&mut self, row: usize, col: usize, value: T) -> Result<(), &'static str> {
        if row < self.rows && col < self.cols {
            self.data[row][col] = value;
            Ok(())
        } else {
            Err("Index out of bounds")
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    pub fn transpose(&self) -> Matrix<T> {
        let mut transposed_data = vec![vec![T::default(); self.rows]; self.cols];
        for i in 0..self.rows {
            for j in 0..self.cols {
                transposed_data[j][i] = self.data[i][j].clone();
            }
        }
        Matrix::from_vec(transposed_data)
    }

    pub fn identity(size: usize) -> Matrix<T> 
    where
        T: From<u8>,
    {
        let mut identity_data = vec![vec![T::default(); size]; size];
        for i in 0..size {
            identity_data[i][i] = T::from(1u8);
        }
        Matrix::from_vec(identity_data)
    }

    pub fn zero(rows: usize, cols: usize) -> Matrix<T> {
        Matrix::new(rows, cols)
    }

    pub fn one(rows: usize, cols: usize) -> Matrix<T> 
    where
        T: From<u8>,
    {
        let one_data = vec![vec![T::from(1u8); cols]; rows];
        Matrix::from_vec(one_data)
    }

    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    pub fn is_symmetric(&self) -> bool 
    where
        T: PartialEq,
    {
        if !self.is_square() {
            return false;
        }
        for i in 0..self.rows {
            for j in 0..i {
                if self.data[i][j] != self.data[j][i] {
                    return false;
                }
            }
        }
        true
    }

    pub fn trace(&self) -> Option<T> 
    where
        T: std::ops::Add<Output = T>,
    {
        if !self.is_square() {
            return None;
        }
        let mut trace = T::default();
        for i in 0..self.rows {
            trace = trace + self.data[i][i].clone();
        }
        Some(trace)
    }

    pub fn determinant(&self) -> Option<T> 
    where
        T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::ops::Mul<Output = T> + From<u8>,
    {
        if !self.is_square() {
            return None;
        }
        let n = self.rows;
        if n == 1 {
            return Some(self.data[0][0].clone());
        }
        if n == 2 {
            return Some(
                self.data[0][0].clone() * self.data[1][1].clone()
                    - self.data[0][1].clone() * self.data[1][0].clone(),
            );
        }
        let mut det = T::default();
        for c in 0..n {
            let mut submatrix_data = vec![];
            for r in 1..n {
                let mut row_data = vec![];
                for cc in 0..n {
                    if cc != c {
                        row_data.push(self.data[r][cc].clone());
                    }
                }
                submatrix_data.push(row_data);
            }
            let submatrix = Matrix::from_vec(submatrix_data);
            let sign = if c % 2 == 0 { T::from(1u8) } else { T::from(0u8) - T::from(1u8) };
            det = det + sign * self.data[0][c].clone() * submatrix.determinant().unwrap();
        }
        Some(det)
    }

    pub fn is_singular(&self) -> Option<bool> 
    where
        T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::ops::Mul<Output = T> + From<u8> + PartialEq,
    {
        match self.determinant() {
            Some(det) => {
                if det == T::from(0u8) {
                    Some(true)
                } else {
                    Some(false)
                }
            }
            None => None,
        }
    }

    pub fn inverse(&self) -> Option<Matrix<T>> 
    where
        T: std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::ops::Mul<Output = T>
            + std::ops::Div<Output = T>
            + From<u8>
            + Default
            + Clone
            + PartialEq,
    {
        if !self.is_square() {
            return None;
        }
        let n = self.rows;
        let mut augmented_data = vec![vec![T::default(); 2 * n]; n];
        for i in 0..n {
            for j in 0..n {
                augmented_data[i][j] = self.data[i][j].clone();
            }
            augmented_data[i][i + n] = T::from(1u8);
        }

        for i in 0..n {
            let pivot = augmented_data[i][i].clone();
            if pivot == T::from(0u8) {
                return None;
            }
            for j in 0..2 * n {
                augmented_data[i][j] = augmented_data[i][j].clone() / pivot.clone();
            }
            for k in 0..n {
                if k != i {
                    let factor = augmented_data[k][i].clone();
                    for j in 0..2 * n {
                        augmented_data[k][j] = augmented_data[k][j].clone()
                            - factor.clone() * augmented_data[i][j].clone();
                    }
                }
            }
        }

        let mut inverse_data = vec![vec![T::default(); n]; n];
        for i in 0..n {
            for j in 0..n {
                inverse_data[i][j] = augmented_data[i][j + n].clone();
            }
        }
        Some(Matrix::from_vec(inverse_data))
    }
}

impl<T> std::ops::Add for Matrix<T>
where
    T: Clone + std::ops::Add<Output = T> + std::default::Default,
{
    type Output = Matrix<T>;

    fn add(self, other: Matrix<T>) -> Matrix<T> {
        if self.rows != other.rows || self.cols != other.cols {
            panic!("Matrix dimensions must agree for addition");
        }
        let mut result_data = vec![vec![T::default(); self.cols]; self.rows];
        for i in 0..self.rows {
            for j in 0..self.cols {
                result_data[i][j] = self.data[i][j].clone() + other.data[i][j].clone();
            }
        }
        Matrix::from_vec(result_data)
    }
}

impl<T> std::ops::Sub for Matrix<T>
where
    T: Clone + std::ops::Sub<Output = T> + std::default::Default,
{
    type Output = Matrix<T>;

    fn sub(self, other: Matrix<T>) -> Matrix<T> {
        if self.rows != other.rows || self.cols != other.cols {
            panic!("Matrix dimensions must agree for subtraction");
        }
        let mut result_data = vec![vec![T::default(); self.cols]; self.rows];
        for i in 0..self.rows {
            for j in 0..self.cols {
                result_data[i][j] = self.data[i][j].clone() - other.data[i][j].clone();
            }
        }
        Matrix::from_vec(result_data)
    }
}

impl<T> std::ops::Mul for Matrix<T>
where
    T: Clone + Default + std::ops::Add<Output = T> + std::ops::Mul<Output = T>,
{
    type Output = Matrix<T>;

    fn mul(self, other: Matrix<T>) -> Matrix<T> {
        if self.cols != other.rows {
            panic!("Matrix dimensions must agree for multiplication");
        }
        let mut result_data = vec![vec![T::default(); other.cols]; self.rows];
        for i in 0..self.rows {
            for j in 0..other.cols {
                for k in 0..self.cols {
                    result_data[i][j] = result_data[i][j].clone()
                        + (self.data[i][k].clone() * other.data[k][j].clone());
                }
            }
        }
        Matrix::from_vec(result_data)
    }
}

impl<T> std::ops::Div for Matrix<T>
where
    T: Clone + std::ops::Div<Output = T> + std::default::Default + PartialEq + std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::ops::Mul<Output = T> + From<u8>,
{
    type Output = Matrix<T>;

    fn div(self, other: Matrix<T>) -> Matrix<T> {
        if self.cols != other.rows {
            panic!("Matrix dimensions must agree for division");
        }
        if let Some(other_inverse) = other.inverse() {
            let result_data = self * other_inverse;
            Matrix::from_vec(result_data.to_vec())
        } else {
            panic!("Matrix is singular, cannot divide");
        }
    }
}