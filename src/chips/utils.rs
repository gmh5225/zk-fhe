use halo2_base::utils::ScalarField;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::identities::Zero;
use num_traits::Num;

/// Performs long polynomial division on two polynomials
/// Returns the quotient and remainder
///
/// * Input polynomials are parsed as a vector of assigned coefficients [a_DEG, a_DEG-1, ..., a_1, a_0] where a_0 is the constant term
/// * DEG_DVD is the degree of the dividend
/// * DEG_DVS is the degree of the divisor
pub fn div_euclid<const DEG_DVD: usize, const DEG_DVS: usize>(
    dividend: &Vec<BigInt>,
    divisor: &Vec<BigInt>,
) -> (Vec<BigInt>, Vec<BigInt>) {
    if divisor.is_empty() || divisor.iter().all(BigInt::is_zero) {
        panic!("Cannot divide by a zero polynomial!");
    }
    if dividend.is_empty() || dividend.iter().all(BigInt::is_zero) {
        return (
            vec![BigInt::zero(); DEG_DVD - DEG_DVS + 1],
            vec![BigInt::zero(); DEG_DVS],
        );
    }

    assert_eq!(dividend.len() - 1, DEG_DVD);
    assert_eq!(divisor.len() - 1, DEG_DVS);

    let mut dividend = dividend.clone();
    let divisor = divisor.clone();
    let mut quotient: Vec<BigInt> = Vec::new();
    let mut remainder: Vec<BigInt> = Vec::new();

    while dividend.len() > divisor.len() - 1 {
        let leading_coefficient_ratio = &dividend[0] / &divisor[0];
        quotient.push(leading_coefficient_ratio.clone());

        for (i, coeff) in divisor.iter().enumerate() {
            dividend[i] = &dividend[i] - &(&leading_coefficient_ratio * coeff);
        }

        dividend.remove(0);
    }

    for coeff in &dividend {
        remainder.push(coeff.clone());
    }

    while !quotient.is_empty() && quotient[0].is_zero() {
        quotient.remove(0);
    }

    while !remainder.is_empty() && remainder[0].is_zero() {
        remainder.remove(0);
    }

    // pad quotient with zeroes at the beginning to make its degree equal to DEG_DVD - DEG_DVS
    let mut quotient = quotient;
    while quotient.len() - 1 < DEG_DVD - DEG_DVS {
        quotient.insert(0, BigInt::from(0u32));
    }

    (quotient, remainder)
}

/// Performs polynomial multiplication on two polynomials using direct method
/// Returns the product of the input polynomials
///
/// * Input polynomials are parsed as a vector of assigned coefficients [a_DEG, a_DEG-1, ..., a_1, a_0] where a_0 is the constant term
pub fn poly_mul(a: &Vec<BigInt>, b: &Vec<BigInt>) -> Vec<BigInt> {
    let deg_a = a.len() - 1;
    let deg_b = b.len() - 1;
    let deg_c = deg_a + deg_b;

    // initialize the output polynomial with zeroes
    let mut c = vec![BigInt::from(0_u64); deg_c + 1];

    // perform polynomial multiplication
    for i in 0..=deg_a {
        for j in 0..=deg_b {
            c[i + j] += a[i].clone() * b[j].clone();
        }
    }

    assert!(c.len() == deg_c + 1);

    c
}

/// Converts a BigInt to a Field Element
pub fn big_uint_to_fp<F: ScalarField>(big_uint: &BigInt) -> F {
    F::from_str_vartime(&big_uint.to_str_radix(10)[..]).unwrap()
}

/// Reduce a polynomial by a modulus Q coefficient-wise
pub fn reduce_poly_by_modulo_q<const Q: u64>(poly: &Vec<BigInt>) -> Vec<BigInt> {
    let mut reduced_poly = Vec::new();

    for coeff in poly {
        let reduced_coeff = coeff.mod_floor(&BigInt::from(Q));
        reduced_poly.push(reduced_coeff);
    }

    reduced_poly
}

/// Transfor Vec<String> to Vec<BigInt>
pub fn vec_string_to_vec_bigint(vec: &Vec<String>) -> Vec<BigInt> {
    let mut vec_bigint = Vec::new();

    for item in vec {
        let bigint = BigInt::from_str_radix(item, 10).unwrap();
        vec_bigint.push(bigint);
    }

    vec_bigint
}
