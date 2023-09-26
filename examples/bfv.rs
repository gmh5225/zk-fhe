use ark_bn254::Fr;
use ark_poly::univariate::DensePolynomial;
use ark_poly::DenseUVPolynomial;
use clap::Parser;
use halo2_base::gates::GateChip;
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
use halo2_base::Context;
use halo2_base::QuantumCell::Constant;
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run_builder_on_inputs;
use serde::{Deserialize, Serialize};
use zk_fhe::chips::poly_operations::{poly_add, poly_mul, poly_scalar_mul};

const N: u64 = 1024;
const Q: u64 = (2 ^ 29) - 3; // 536870909
const T: u64 = 7;
const B: u64 = 30;

/// Circuit inputs for BFV encryption operations
///
/// # Type Parameters
///
/// * `N`: Degree of the cyclotomic polynomial
/// * `Q`: Modulus of the cipher text field
/// * `T`: Modulus of the plaintext field
/// * `DELTA` : Q/T rounded to the lower integer
/// * `B`: Upper bound of the Gaussian distribution. It is defined as 6 * sigma
///
/// # Fields
///
/// * `pk0`: Public key 0 polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i
/// * `pk1`: Public key 1 polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i
/// * `m`: Plaintext polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Represents the message to be encrypted
/// * `u`: Ephemeral key polynomial coefficients from the distribution ChiKey
/// * `e0`: Error polynomial coefficients from the distribution ChiError
/// * `e1`: Error polynomial coefficients from the distribution ChiError

///
/// # Assumes that the following checks have been performed outside the circuit
/// - `N` must be a power of 2
/// - `Q` must be a prime number
/// - `Q` must be greater than 1.
/// -  If n is the number of bits of Q, and m is the number of bits of the prime field of the circuit. n must be set such that (n * 2) + 2 < m to avoid overflow of the coefficients of the polynomials
/// - `T` must be a prime number and must be greater than 1 and less than `Q`
/// - `B` must be a positive integer
/// - `pk0` and `pk1` must be polynomials in the R_q ring

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: u64, const Q: u64, const T: u64, const B: u64> {
    pub pk0: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_q
    pub pk1: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_q
    pub m: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_t
    pub u: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_q
    pub e0: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_q
    pub e1: Vec<u64>, // polynomial coefficients [a_0, a_1, ..., a_N] where a_i is the coefficient of x^i. Lives in R_q
}

fn bfv_encryption_circuit<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N, Q, T, B>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // Assign the input polynomials to the circuit
    let pk0: Vec<AssignedValue<F>> = input
        .pk0
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect();

    let pk1: Vec<AssignedValue<F>> = input
        .pk1
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect();

    let u: Vec<AssignedValue<F>> = input
        .u
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect();

    let m = input
        .m
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect::<Vec<AssignedValue<F>>>();

    let e0 = input
        .e0
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect::<Vec<AssignedValue<F>>>();

    let e1 = input.e1.iter().map(|x| F::from(*x)).collect::<Vec<F>>();

    // TO DO: Check if e0 and e1 are correcly sampled from the distribution ChiError
    // TO DO: Check if u is correcly sampled from the distribution ChiKey
    // TO DO: Check if m belongs to the R_t ring
    // TO DO: Check if e0, e1 and u are polynomials in the R_q ring

    let gate = GateChip::<F>::default();

    // COMPUTE C0

    // pk0 * u
    // The coefficients of pk0 are in the range [0, Q) according to the check to be performed outside the circuit. If Q has n bits, pk0 can have at most n bits.
    // The coefficients of u are in the range [0, B] [Q-B, Q-1] according to check performed inside the circuit. If Q has n bits, u can have at most n bits.
    // The expansion rate of the coefficients of pk0_u is 2n bits.
    // If n = 29 bits, the maximum expansion of the coefficients of pk0_u is 58 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when multiplying pk0 by u
    let pk0_u = poly_mul::<N, F>(ctx, pk0, u.clone(), &gate);

    // TO DO: reduce the coefficients of pk0_u by the cyclotomic polynomial of degree `N` => x^N + 1.
    // By doing this, pk0_u will be reduced to a polynomial of degree `N`
    const DELTA: u64 = Q / T;

    // m * delta
    // The coefficients of m are in the range [0, T) according to the check performed inside the circuit.
    // Delta is a constant in the range [0, Q) as it is defined as Q/T rounded to the lower integer and T < Q and T > 1
    // If both Q and T have n bits (in practice T is much smaller than Q), the expansion rate of the coefficients of m_delta is 2n bits.
    // If n = 29 bits, the maximum expansion of the coefficients of m_delta is 58 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when multiplying m by delta
    let m_delta = poly_scalar_mul::<N, F>(ctx, m, Constant(F::from(DELTA)), &gate);

    // TO DO: perform pk0 * u + m * delta

    // The coefficients of pk0_u are in the [0, 2^2n) range according to the operations performed above (where n is the number of bits of Q)
    // The coefficients of m_delta are in the [0, 2^2n) range according to the operations performed above (where n is the number of bits of Q)
    // If both pk0_u and m_delta have 2n bits, the expansion rate of the coefficients of pk0_u_plus_m_delta is 2n + 1 bits.
    // If n = 29 bits, the maximum expansion of the coefficients of pk0_u_plus_m_delta is 59 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when adding pk0_u by m_delta
    // let pk0_u_plus_m_delta = poly_add::<N, F>(ctx, pk0_u, m_delta, &gate);

    // TO DO: perform pk0 * u + m * delta + e0 to get c0

    // The coefficients of pk0_u_plus_m_delta are in the [0, 2^2n+1) range according to the operations performed above (where n is the number of bits of Q)
    // The coefficients of e0 are either [0, 1, Q-1] according to the check performed inside the circuit. The maximum value of a coefficient of e0 is n bits.
    // If pk0_u_plus_m_delta has 2n+1 bits and e0 has n bits, the expansion rate of the coefficients of c0 is 2n + 1 + 1 bits.
    // If n = 29 bits, the maximum expansion of the coefficients of c0 is 60 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when adding pk0_u_plus_m_delta by e0

    // let c0 = poly_add::<N, F>(ctx, pk0_u_plus_m_delta, e0, &gate);

    // TO DO: reduce the cofficients of c0 by modulo `Q`
    // TO DO: further reduce the coefficients of c0 by the cyclotomic polynomial of degree `N` => x^N + 1

    // COMPUTE C1

    // pk1 * u
    // The coefficients of pk1 are in the range [0, Q) according to the check to be performed outside the circuit. If Q has n bits, pk1 can have at most n bits.
    // The coefficients of u are in the range [0, B] [Q-B, Q-1] according to check performed inside the circuit. If Q has n bits, u can have at most n bits.
    // The expansion rate of the coefficients of pk1_u is 2n bits.
    // If n = 29 bits, the maximum expansion of the coefficients of pk1_u is 58 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when multiplying pk1 by u
    let pk1_u = poly_mul::<N, F>(ctx, pk1, u, &gate);

    // TO DO: reduce the coefficients of pk1_u by the cyclotomic polynomial of degree `N` => x^N + 1.
    // By doing this, pk1_u will be reduced to a polynomial of degree `N`

    // The coefficients of pk1_u are in the [0, 2^2n) range according to the operations performed above (where n is the number of bits of Q)
    // The coefficients of e1 are either [0, 1, Q-1] according to the check performed inside the circuit. The maximum value of a coefficient of e1 is n bits.
    // If pk1_u has 2n bits and e0 has n bits, the expansion rate of the coefficients of c1 is 2n + 1 bits.
    // If n = 29 bits, the maximum expansion of the coefficients of c0 is 59 bits, which is below the prime field of the circuit (254 bits)
    // No risk of coefficients overflowing the circuit prime field when adding pk0_u_plus_m_delta by e0

    // TO DO: perform pk1 * u + e1 to get c1
    // let c1 = poly_add::<N, F>(ctx, pk1_u, e1, &gate);

    // TO DO: reduce the cofficients of c0 by modulo `Q`
    // TO DO: further reduce the coefficients of c0 by the cyclotomic polynomial of degree `N` => x^N + 1

    // TO DO: Expose to the public the coefficients of c0 and c1
    // TO DO: Expose to the public pk0 and pk1
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // create polynomials pk0 and pk1 of degree N with random coefficients in the range [0, Q)
    let pk0 = (0..N + 1)
        .map(|_| rand::random::<u64>() % Q)
        .collect::<Vec<u64>>();

    let pk1 = (0..N + 1)
        .map(|_| rand::random::<u64>() % Q)
        .collect::<Vec<u64>>();

    // create polynomial m of degree N with random coefficients in the range [0, T)
    let m = (0..N + 1)
        .map(|_| rand::random::<u64>() % T)
        .collect::<Vec<u64>>();

    // create polynomial u of degree N with random coefficients in the range [0, B]
    let u = (0..N + 1)
        .map(|_| rand::random::<u64>() % B)
        .collect::<Vec<u64>>();

    // create polynomial e0 of degree N with random coefficients in the range [0, 1]
    let e0 = (0..N + 1)
        .map(|_| rand::random::<u64>() % 2)
        .collect::<Vec<u64>>();

    // create polynomial e1 of degree N with random coefficients in the range [0, 1]
    let e1 = (0..N + 1)
        .map(|_| rand::random::<u64>() % 2)
        .collect::<Vec<u64>>();

    let input = CircuitInput::<N, Q, T, B> {
        pk0,
        pk1,
        m,
        u,
        e0,
        e1,
    };

    // run different zk commands based on the command line arguments
    run_builder_on_inputs(
        |builder, input, public| bfv_encryption_circuit(builder.main(0), input, public),
        args,
        input,
    );
}
