use num_traits::Float;
use std::iter::{Sum,Product};
use std::convert::From;
pub trait StatisticsTrait : Float + Sum + From<f64> + Ord + Copy + Product{}

pub fn mean<T: StatisticsTrait>(data: Vec<T>) -> T {
    let sum: T = data.iter().cloned().sum();
    sum / (data.len() as f64).into()
}

pub fn harmonic_mean<T: StatisticsTrait>(data: Vec<T>) -> T {
    let n : T = (data.len() as f64).into();
    let sum_reciprocal: T = data.iter().map(|&x| T::one() / x).sum();
    n / sum_reciprocal
}

pub fn square_mean<T: StatisticsTrait>(data: Vec<T>) -> T {
    let sum_squares: T = data.iter().map(|&x| x * x).sum();
    sum_squares / (data.len() as f64).into()
}

pub fn geometric_mean<T: StatisticsTrait>(data: Vec<T>) -> T {
    let product: T = data.iter().cloned().product();
    product.powf((1.0 / (data.len() as f64)).into())
}

pub fn variance<T: StatisticsTrait>(data: Vec<T>, mean: T) -> T {
    let var_sum: T = data.iter().map(|&x| {
        let diff = x - mean;
        diff * diff
    }).sum();
    var_sum / (data.len() as f64).into()
}

pub fn std_deviation<T: StatisticsTrait>(data: Vec<T>, mean: T) -> T {
    variance(data, mean).sqrt()
}

pub fn covariance<T: StatisticsTrait>(data_x: Vec<T>, data_y: Vec<T>) -> T {
    let mean_x = mean(data_x.clone());
    let mean_y = mean(data_y.clone());
    let cov_sum: T = data_x.iter().zip(data_y.iter()).map(|(&x, &y)| {
        (x - mean_x) * (y - mean_y)
    }).sum();
    cov_sum / (data_x.len() as f64).into()
}

pub fn median<T: StatisticsTrait>(data: &mut Vec<T>) -> T {
    data.sort();
    let len = data.len();
    if len % 2 == 0 {
        let mid1 = data[len / 2 - 1];
        let mid2 = data[len / 2];
        (mid1 + mid2) / (2.0).into()
    } else {
        data[len / 2]
    }
}

pub fn percentile<T: StatisticsTrait>(data: &mut Vec<T>, percentile: f64) -> T {
    data.sort();
    let k = (percentile / 100.0) * ((data.len() - 1) as f64);
    let f = k.floor() as usize;
    let c = k.ceil() as usize;
    if f == c {
        data[f]
    } else {
        let d0 = data[f] * ((c as f64 - k).into());
        let d1 = data[c] * ((k - f as f64).into());
        d0 + d1
    }
}

pub fn unique<T>(data: &Vec<T>) -> Vec<T> 
where
    T: std::cmp::Eq + std::hash::Hash + Clone,
{
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    data.iter().filter(|&x| seen.insert(x.clone())).cloned().collect()
}