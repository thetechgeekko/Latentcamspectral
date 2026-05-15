use rand::Rng;
use rand_distr::{StandardNormal, Distribution};

pub fn fast_binomial_scalar(n: i64, p: f64, rng: &mut impl Rng) -> i64 {
    if p <= 0.0 {
        return 0;
    } else if p >= 1.0 {
        return n;
    }
    
    if n < 25 {
        let mut count = 0;
        for _ in 0..n {
            if rng.gen::<f64>() < p {
                count += 1;
            }
        }
        count
    } else {
        let mean = n as f64 * p;
        let var = mean * (1.0 - p);
        if var > 10.0 {
            let z: f64 = StandardNormal.sample(rng);
            let approx = mean + var.sqrt() * z;
            let approx_int = approx.round() as i64;
            approx_int.clamp(0, n)
        } else {
            let u: f64 = rng.gen();
            let mut cdf = 0.0;
            let mut prob_f = (1.0 - p).powf(n as f64);
            let mut k = 0;
            while cdf < u && k <= n {
                cdf += prob_f;
                if k < n {
                    prob_f = prob_f * ((n - k) as f64 / (k + 1) as f64) * (p / (1.0 - p));
                }
                k += 1;
            }
            k - 1
        }
    }
}

pub fn fast_poisson_scalar(lam: f64, rng: &mut impl Rng) -> i64 {
    if lam <= 0.0 {
        return 0;
    } else if lam < 30.0 {
        let l = (-lam).exp();
        let mut p_val = 1.0;
        let mut k = 0;
        while p_val > l {
            k += 1;
            p_val *= rng.gen::<f64>();
        }
        k - 1
    } else {
        let z: f64 = StandardNormal.sample(rng);
        let sample = lam + lam.sqrt() * z;
        let sample_int = sample.round() as i64;
        sample_int.max(0)
    }
}

pub fn fast_lognormal_scalar(mu: f64, sigma: f64, rng: &mut impl Rng) -> f64 {
    if sigma < 1e-6 {
        mu.exp()
    } else {
        let z: f64 = StandardNormal.sample(rng);
        (mu + sigma * z).exp()
    }
}

pub fn fast_lognormal_from_mean_std_scalar(m: f64, s: f64, rng: &mut impl Rng) -> f64 {
    if m <= 0.0 {
        fast_lognormal_scalar(0.0, 0.0, rng)
    } else {
        let sigma2 = (1.0 + (s * s) / (m * m)).ln();
        let sigma = sigma2.sqrt();
        let mu = m.ln() - sigma2 / 2.0;
        fast_lognormal_scalar(mu, sigma, rng)
    }
}
