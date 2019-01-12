//! Low-level manipulations of IEEE754 floating-point numbers.
//!
//! # Installation
//!
//! Add this to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! ieee754 = "0.2"
//! ```
//!
//! # Examples
//!
//! ```rust
//! use ieee754::Ieee754;
//!
//! // there are 840 single-precision floats between 1.0 and 1.0001
//! // (inclusive).
//! assert_eq!(1_f32.upto(1.0001).count(), 840);
//! ```

#![no_std]
#[cfg(test)] #[macro_use] extern crate std;

use core::mem;
use core::cmp::Ordering;

/// An iterator over floating point numbers, created by `Ieee754::upto`.
pub struct Iter<T: Ieee754> {
    from: T,
    to: T,
    done: bool
}
impl<T: Ieee754> Iterator for Iter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.done { return None }

        let x = self.from;
        let y = x.next();
        // we've canonicalised negative zero to positive zero, and
        // we're guaranteed that neither is NaN, so comparing bitwise
        // is valid (and 20% faster for the `all` example).
        if x.bits() == self.to.bits() {
            self.done = true;
        }
        self.from = y;
        return Some(x)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done {
            return (0, Some(0))
        }

        let high_pos = 8 * mem::size_of::<T>() - 1;
        let high_mask = 1 << high_pos;

        let from_ = self.from.bits().as_u64();
        let (from, from_sign) = (from_ & !high_mask,
                                 from_ & high_mask != 0);
        let to_ = self.to.bits().as_u64();
        let (to, to_sign) = (to_ & !high_mask,
                             to_ & high_mask != 0);
        let from = if from_sign { -(from as i64) } else { from as i64 };
        let to = if to_sign { -(to as i64) } else { to as i64 };

        let distance = (to - from + 1) as u64;
        if distance <= core::usize::MAX as u64 {
            let d = distance as usize;
            (d, Some(d))
        } else {
            (core::usize::MAX, None)
        }
    }
}
impl<T: Ieee754> DoubleEndedIterator for Iter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.done { return None }

        let x = self.to;
        let y = x.prev();
        if x == self.from {
            self.done = true;
        }
        self.to = y;
        return Some(x)
    }
}

pub trait Bits: Eq + PartialEq + PartialOrd + Ord + Copy {
    fn as_u64(self) -> u64;
}
impl Bits for u32 {
    fn as_u64(self) -> u64 { self as u64 }
}
impl Bits for u64 {
    fn as_u64(self) -> u64 { self }
}

/// Types that are IEEE754 floating point numbers.
pub trait Ieee754: Copy + PartialEq + PartialOrd {
    /// Iterate over each value of `Self` in `[self, lim]`.
    ///
    /// The returned iterator will include subnormal numbers, and will
    /// only include one of `-0.0` and `0.0`.
    ///
    /// # Panics
    ///
    /// Panics if `self > lim`, or if either are NaN.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// // there are 840 single-precision floats in between 1.0 and 1.0001
    /// // (inclusive).
    /// assert_eq!(1_f32.upto(1.0001).count(), 840);
    /// ```
    fn upto(self, lim: Self) -> Iter<Self>;

    /// A type that represents the raw bits of `Self`.
    type Bits: Bits;
    /// A type large enough to store the true exponent of `Self`.
    type Exponent;
    /// A type large enough to store the raw exponent (i.e. with the bias).
    type RawExponent;
    /// A type large enough to store the significand of `Self`.
    type Significand;

    /// Return the next value after `self`.
    ///
    /// Calling this on NaN or positive infinity will yield nonsense.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    /// let x: f32 = 1.0;
    /// assert_eq!(x.next(), 1.000000119209);
    /// ```
    fn next(self) -> Self;

    /// Return the previous value before `self`.
    ///
    /// Calling this on NaN or negative infinity will yield nonsense.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    /// let x: f32 = 1.0;
    /// assert_eq!(x.prev(), 0.99999995);
    /// ```
    fn prev(self) -> Self;

    /// Return the unit-in-the-last-place ulp of `self`. That is,
    /// `x.abs().next() - x.abs()`, but handling overflow properly.
    ///
    /// Returns `None` if `self` is not finite.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use std::f32;
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(0_f32.ulp(), Some(1.4e-45));
    ///
    /// assert_eq!(1_f32.ulp(), Some(1.1920928955078125e-07));
    /// assert_eq!((-1_f32).ulp(), Some(1.1920928955078125e-07));
    ///
    /// // 2^23
    /// assert_eq!(8_388_608_f32.ulp(), Some(1.0));
    /// // 2^24 - 1, the largest f32 with ULP 1
    /// assert_eq!(16_777_215_f32.ulp(), Some(1.0));
    /// // 2^24
    /// assert_eq!(16_777_216_f32.ulp(), Some(2.0));
    ///
    /// // non-finite
    /// assert_eq!(f32::INFINITY.ulp(), None);
    /// assert_eq!(f32::NAN.ulp(), None);
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use std::f64;
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(0_f64.ulp(), Some(4.9e-324));
    ///
    /// assert_eq!(1_f64.ulp(), Some(2.220446049250313e-16));
    /// assert_eq!((-1_f64).ulp(), Some(2.220446049250313e-16));
    ///
    /// // 2^52
    /// assert_eq!(4_503_599_627_370_496_f64.ulp(), Some(1.0));
    /// // 2^53 - 1, the largest f64 with ULP 1
    /// assert_eq!(9_007_199_254_740_991_f64.ulp(), Some(1.0));
    /// // 2^53
    /// assert_eq!(9_007_199_254_740_992_f64.ulp(), Some(2.0));
    ///
    /// // non-finite
    /// assert_eq!(f64::INFINITY.ulp(), None);
    /// assert_eq!(f64::NAN.ulp(), None);
    /// ```
    fn ulp(self) -> Option<Self>;

    /// View `self` as a collection of bits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    /// let x: f32 = 1.0;
    /// assert_eq!(x.bits(), 0x3f80_0000);
    /// ```
    fn bits(self) -> Self::Bits;

    /// View a collections of bits as a floating point number.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    /// let float: f32 = Ieee754::from_bits(0xbf80_0000);
    /// assert_eq!(float, -1.0);
    /// ```
    fn from_bits(x: Self::Bits) -> Self;

    /// Get the bias of the stored exponent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(f32::exponent_bias(), 127);
    /// assert_eq!(f64::exponent_bias(), 1023);
    /// ```
    fn exponent_bias() -> Self::Exponent;

    /// Break `self` into the three constituent parts of an IEEE754 float.
    ///
    /// The exponent returned is the raw bits, use `exponent_bias` to
    /// compute the offset required or use `decompose` to obtain this
    /// in precomputed form.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(1_f32.decompose_raw(), (false, 127, 0));
    /// assert_eq!(1234.567_f32.decompose_raw(), (false, 137, 0x1a5225));
    ///
    /// assert_eq!((-0.525_f32).decompose_raw(), (true, 126, 0x66666));
    ///
    /// assert_eq!(std::f32::INFINITY.decompose_raw(), (false, 255, 0));
    ///
    /// let (sign, expn, signif) = std::f32::NAN.decompose_raw();
    /// assert_eq!((sign, expn), (false, 255));
    /// assert!(signif != 0);
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(1_f64.decompose_raw(), (false, 1023, 0));
    /// assert_eq!(1234.567_f64.decompose_raw(), (false, 1033, 0x34a449ba5e354));
    ///
    /// assert_eq!((-0.525_f64).decompose_raw(), (true, 1022, 0xcccc_cccc_cccd));
    ///
    /// assert_eq!(std::f64::INFINITY.decompose_raw(), (false, 2047, 0));
    ///
    /// let (sign, expn, signif) = std::f64::NAN.decompose_raw();
    /// assert_eq!((sign, expn), (false, 2047));
    /// assert!(signif != 0);
    /// ```
    fn decompose_raw(self) -> (bool, Self::RawExponent, Self::Significand);

    /// Create a `Self` out of the three constituent parts of an IEEE754 float.
    ///
    /// This returns (-1)<sup><code>sign</code></sup> ×
    /// 1.<code>signif</code> × 2<sup><code>expn</code> - bias</sup>, where
    ///
    /// - `sign` is treated as if `true` == `1` (meaning `true` is
    ///   negative),
    /// - 1.<code>signif</code> refers to placing the bits of `signif`
    ///   as the fractional part of a number between 1 and 2, and
    /// - bias is the exponent bias for this float (see [`exponent_bias`]).
    ///
    /// The exponent should be the raw bits: use `exponent_bias` to
    /// compute the offset required, or use `recompose` to feed in an
    /// unbiased exponent.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(f32::recompose_raw(false, 127, 0), 1.0);
    /// assert_eq!(f32::recompose_raw(false, 137, 0x1a5225), 1234.567);
    /// assert_eq!(f32::recompose_raw(true, 126, 0x66666), -0.525);
    ///
    /// assert_eq!(f32::recompose_raw(false, 255, 0), std::f32::INFINITY);
    ///
    /// assert!(f32::recompose_raw(false, 255, 1).is_nan());
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(f64::recompose_raw(false, 1023, 0), 1.0);
    /// assert_eq!(f64::recompose_raw(false, 1033, 0x34a449ba5e354), 1234.567);
    /// assert_eq!(f64::recompose_raw(true, 1022, 0xcccc_cccc_cccd), -0.525);
    ///
    /// assert_eq!(f64::recompose_raw(false, 2047, 0), std::f64::INFINITY);
    ///
    /// assert!(f64::recompose_raw(false, 2047, 1).is_nan());
    /// ```
    fn recompose_raw(sign: bool, expn: Self::RawExponent, signif: Self::Significand) -> Self;

    /// Break `self` into the three constituent parts of an IEEE754 float.
    ///
    /// The exponent returned is the true exponent, after accounting
    /// for the bias it is stored with. The significand does not
    /// include the implicit highest bit (if it exists), e.g. the
    /// 24-bit for single precision.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(1_f32.decompose(), (false, 0, 0));
    /// assert_eq!(1234.567_f32.decompose(), (false, 10, 0x1a5225));
    ///
    /// assert_eq!((-0.525_f32).decompose(), (true, -1, 0x66666));
    ///
    /// assert_eq!(std::f32::INFINITY.decompose(), (false, 128, 0));
    ///
    /// let (sign, expn, signif) = std::f32::NAN.decompose();
    /// assert_eq!((sign, expn), (false, 128));
    /// assert!(signif != 0);
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(1_f64.decompose(), (false, 0, 0));
    /// assert_eq!(1234.567_f64.decompose(), (false, 10, 0x34a449ba5e354));
    ///
    /// assert_eq!((-0.525_f64).decompose(), (true, -1, 0xcccc_cccc_cccd));
    ///
    /// assert_eq!(std::f64::INFINITY.decompose(), (false, 1024, 0));
    ///
    /// let (sign, expn, signif) = std::f64::NAN.decompose();
    /// assert_eq!((sign, expn), (false, 1024));
    /// assert!(signif != 0);
    /// ```
    fn decompose(self) -> (bool, Self::Exponent, Self::Significand);

    /// Create a `Self` out of the three constituent parts of an IEEE754 float.
    ///
    /// This returns (-1)<sup><code>sign</code></sup> ×
    /// 1.<code>signif</code> × 2<sup><code>expn</code></sup>, where
    ///
    /// - `sign` is treated as if `true` == `1` (meaning `true` is
    ///   negative), and
    /// - 1.<code>signif</code> refers to placing the bits of `signif`
    ///   as the fractional part of a number between 1 and 2.
    ///
    /// The exponent should be the true exponent, not accounting for any
    /// bias. The significand should not include the implicit highest
    /// bit (if it exists), e.g. the 24-th bit for single precision.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// // normal numbers
    /// assert_eq!(f32::recompose(false, 0, 0), 1.0);
    /// assert_eq!(f32::recompose(false, 10, 0x1a5225), 1234.567);
    /// assert_eq!(f32::recompose(true, -1, 0x66666), -0.525);
    ///
    /// // infinity
    /// assert_eq!(f32::recompose(false, 128, 0), std::f32::INFINITY);
    ///
    /// // NaN
    /// assert!(f32::recompose(false, 128, 1).is_nan());
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use ieee754::Ieee754;
    ///
    /// // normal numbers
    /// assert_eq!(f64::recompose(false, 0, 0), 1.0);
    /// assert_eq!(f64::recompose(false, 10, 0x34a449ba5e354), 1234.567);
    /// assert_eq!(f64::recompose(true, -1, 0xcccc_cccc_cccd), -0.525);
    ///
    /// // infinity
    /// assert_eq!(f64::recompose(false, 1024, 0), std::f64::INFINITY);
    ///
    /// // NaN
    /// assert!(f64::recompose(false, 1024, 1).is_nan());
    /// ```
    fn recompose(sign: bool, expn: Self::Exponent, signif: Self::Significand) -> Self;

    /// Compare `x` and `y` using the IEEE-754 `totalOrder` predicate
    /// (Section 5.10).
    ///
    /// This orders NaNs before or after all non-NaN floats, depending
    /// on the sign bit. Using -qNaN to represent a quiet NaN with
    /// negative sign bit and similarly for a signalling NaN (sNaN),
    /// the order is:
    ///
    /// ```txt
    /// -qNaN < -sNaN < -∞ < -12.34 < -0.0 < +0.0 < +12.34 < +∞ < +sNaN < +qNaN
    /// ```
    ///
    /// (NaNs are ordered according to their payload.)
    ///
    /// # Examples
    ///
    /// Sorting:
    ///
    /// ```rust
    /// use std::f32;
    ///
    /// use ieee754::Ieee754;
    ///
    /// let mut data = vec![0.0, f32::NEG_INFINITY, -1.0, f32::INFINITY,
    ///                     f32::NAN, -0.0, 12.34e5, -f32::NAN];
    /// data.sort_by(|a, b| a.total_cmp(b));
    ///
    /// assert_eq!(format!("{:.0?}", data),
    ///            "[NaN, -inf, -1, -0, 0, 1234000, inf, NaN]");
    /// ```
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use std::cmp::Ordering;
    /// use std::f32;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // normal comparison
    /// assert_eq!(0_f32.total_cmp(&0_f32), Ordering::Equal);
    /// assert_eq!(0_f32.total_cmp(&1_f32), Ordering::Less);
    /// assert_eq!(1e10_f32.total_cmp(&f32::NEG_INFINITY), Ordering::Greater);
    ///
    /// // signed zero
    /// assert_eq!(0_f32.total_cmp(&-0_f32), Ordering::Greater);
    ///
    /// // NaNs
    /// assert_eq!(f32::NAN.total_cmp(&0_f32), Ordering::Greater);
    /// assert_eq!(f32::NAN.total_cmp(&f32::INFINITY), Ordering::Greater);
    /// assert_eq!((-f32::NAN).total_cmp(&f32::NEG_INFINITY), Ordering::Less);
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use std::cmp::Ordering;
    /// use std::f64;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // normal comparison
    /// assert_eq!(0_f64.total_cmp(&0_f64), Ordering::Equal);
    /// assert_eq!(0_f64.total_cmp(&1_f64), Ordering::Less);
    /// assert_eq!(1e10_f64.total_cmp(&f64::NEG_INFINITY), Ordering::Greater);
    ///
    /// // signed zero
    /// assert_eq!(0_f64.total_cmp(&-0_f64), Ordering::Greater);
    ///
    /// // NaNs
    /// assert_eq!(f64::NAN.total_cmp(&0_f64), Ordering::Greater);
    /// assert_eq!(f64::NAN.total_cmp(&f64::INFINITY), Ordering::Greater);
    /// assert_eq!((-f64::NAN).total_cmp(&f64::NEG_INFINITY), Ordering::Less);
    /// ```
    fn total_cmp(&self, other: &Self) -> Ordering;

    /// Return the absolute value of `x`.
    ///
    /// This provides a no_std/core-only version of the built-in `abs` in
    /// `std`, until
    /// [#50145](https://github.com/rust-lang/rust/issues/50145) is
    /// addressed.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// #![no_std]
    /// # extern crate std; // this makes this "test" a lie, unfortunately
    /// # extern crate ieee754;
    /// use core::f32;
    ///
    /// use ieee754::Ieee754;
    ///
    /// # fn main() {
    /// assert_eq!((0_f32).abs(), 0.0);
    ///
    /// assert_eq!((12.34_f32).abs(), 12.34);
    /// assert_eq!((-12.34_f32).abs(), 12.34);
    ///
    /// assert_eq!(f32::INFINITY.abs(), f32::INFINITY);
    /// assert_eq!(f32::NEG_INFINITY.abs(), f32::INFINITY);
    /// assert!(f32::NAN.abs().is_nan());
    /// # }
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// #![no_std]
    /// # extern crate std; // this makes this "test" a lie, unfortunately
    /// # extern crate ieee754;
    /// use core::f64;
    ///
    /// use ieee754::Ieee754;
    ///
    /// # fn main() {
    /// assert_eq!((0_f64).abs(), 0.0);
    ///
    /// assert_eq!((12.34_f64).abs(), 12.34);
    /// assert_eq!((-12.34_f64).abs(), 12.34);
    ///
    /// assert_eq!(f64::INFINITY.abs(), f64::INFINITY);
    /// assert_eq!(f64::NEG_INFINITY.abs(), f64::INFINITY);
    /// assert!(f64::NAN.abs().is_nan());
    /// # }
    /// ```
    fn abs(self) -> Self;

    /// Return a float with the magnitude of `self` but the sign of
    /// `sign`.
    ///
    /// If `sign` is NaN, this still uses its sign bit, and does not
    /// (necessarily) return NaN.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use std::f32;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // normal numbers
    /// assert_eq!(1_f32.copy_sign(1.0), 1.0);
    /// assert_eq!(2_f32.copy_sign(-1.0), -2.0);
    /// assert_eq!((-3_f32).copy_sign(1.0), 3.0);
    /// assert_eq!((-4_f32).copy_sign(-1.0), -4.0);
    ///
    /// // infinities
    /// assert_eq!(5_f32.copy_sign(f32::NEG_INFINITY), -5.0);
    /// assert_eq!(f32::NEG_INFINITY.copy_sign(1.0), f32::INFINITY);
    ///
    /// // signs of zeros matter
    /// assert_eq!((-6_f32).copy_sign(0.0), 6.0);
    /// assert_eq!(7_f32.copy_sign(-0.0), -7.0);
    ///
    /// // NaNs only propagate on the self argument
    /// assert!(f32::NAN.copy_sign(1.0).is_nan());
    /// assert_eq!(8_f32.copy_sign(-f32::NAN), -8.0);
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use std::f64;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // normal numbers
    /// assert_eq!(1_f64.copy_sign(1.0), 1.0);
    /// assert_eq!(2_f64.copy_sign(-1.0), -2.0);
    /// assert_eq!((-3_f64).copy_sign(1.0), 3.0);
    /// assert_eq!((-4_f64).copy_sign(-1.0), -4.0);
    ///
    /// // infinities
    /// assert_eq!(5_f64.copy_sign(f64::NEG_INFINITY), -5.0);
    /// assert_eq!(f64::NEG_INFINITY.copy_sign(1.0), f64::INFINITY);
    ///
    /// // signs of zeros matter
    /// assert_eq!((-6_f64).copy_sign(0.0), 6.0);
    /// assert_eq!(7_f64.copy_sign(-0.0), -7.0);
    ///
    /// // NaNs only propagate on the self argument
    /// assert!(f64::NAN.copy_sign(1.0).is_nan());
    /// assert_eq!(8_f64.copy_sign(-f64::NAN), -8.0);
    /// ```
    fn copy_sign(self, sign: Self) -> Self;

    /// Return the sign of `x`.
    ///
    /// This provides a no_std/core-only function similar to the
    /// built-in `signum` in `std` (until
    /// [#50145](https://github.com/rust-lang/rust/issues/50145) is
    /// addressed). This `sign` function differs at two values; it
    /// matches the mathematical definitions when `self == 0.0` :
    ///
    /// | `x` | `x.signum()` (`std`) | `x.sign()` (`ieee754`) |
    /// |--:|--:|--:|
    /// |< 0.0|−1.0|−1.0|
    /// |−0.0|−1.0|**−0.0**|
    /// |+0.0|+1.0|**+0.0**|
    /// |> 0.0|+1.0|+1.0|
    /// |NaN|NaN|NaN|
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use std::f32;
    /// use std::cmp::Ordering;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // zeros
    /// assert_eq!(0_f32.sign().total_cmp(&0.0), Ordering::Equal);
    /// assert_eq!((-0_f32).sign().total_cmp(&-0.0), Ordering::Equal);
    ///
    /// // normal numbers
    /// assert_eq!((12.34_f32).sign(), 1.0);
    /// assert_eq!((-12.34_f32).sign(), -1.0);
    ///
    /// // extremes
    /// assert_eq!(f32::INFINITY.sign(), 1.0);
    /// assert_eq!(f32::NEG_INFINITY.sign(), -1.0);
    /// assert!(f32::NAN.sign().is_nan());
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use std::f64;
    /// use std::cmp::Ordering;
    ///
    /// use ieee754::Ieee754;
    ///
    /// // zeros
    /// assert_eq!(0_f64.sign().total_cmp(&0.0), Ordering::Equal);
    /// assert_eq!((-0_f64).sign().total_cmp(&-0.0), Ordering::Equal);
    ///
    /// // normal numbers
    /// assert_eq!((12.34_f64).sign(), 1.0);
    /// assert_eq!((-12.34_f64).sign(), -1.0);
    ///
    /// // extremes
    /// assert_eq!(f64::INFINITY.sign(), 1.0);
    /// assert_eq!(f64::NEG_INFINITY.sign(), -1.0);
    /// assert!(f64::NAN.sign().is_nan());
    /// ```
    fn sign(self) -> Self;

    /// Compute the (generalized) **signed** relative error of `self`
    /// as an approximation to `exact`.
    ///
    /// This computes the signed value: positive indicates `self` in
    /// the opposite direction to 0 from `exact`; negative indicates
    /// `self` is in the same direction as 0 from `exact`. Use
    /// `x.rel_error(exact).abs()` to get the non-signed relative
    /// error.
    ///
    /// The "generalized" refers to `exact` being 0 or ±∞ the handling
    /// of which is designed to indicate a "failure" (infinite error),
    /// if `self` doesn't precisely equal `exact`. This behaviour is
    /// designed for checking output of algorithms on floats when it
    /// is often desirable to match 0.0 and ±∞ perfectly.
    ///
    /// The values of this function are:
    ///
    /// |`exact`|`x`|`x.rel_error(exact)`|
    /// |--:|--:|--:|
    /// |NaN|any value|NaN|
    /// |any value|NaN|NaN|
    /// |0|equal to `exact`|0|
    /// |0|not equal to `exact`|signum(`x`) × ∞|
    /// |±∞|equal to `exact`|0|
    /// |±∞|not equal to `exact`|-∞|
    /// |any other value|any value|`(x - exact) / exact`|
    ///
    /// The sign of a zero-valued argument has no effect on the result
    /// of this function.
    ///
    /// # Examples
    ///
    /// Single precision:
    ///
    /// ```rust
    /// use std::f32;
    ///
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(4_f32.rel_error(4.0), 0.0);
    /// assert_eq!(3_f32.rel_error(4.0), -0.25);
    /// assert_eq!(5_f32.rel_error(4.0), 0.25);
    ///
    /// // zero
    /// assert_eq!(0_f32.rel_error(0.0), 0.0);
    /// assert_eq!(1_f32.rel_error(0.0), f32::INFINITY);
    /// assert_eq!((-1_f32).rel_error(0.0), f32::NEG_INFINITY);
    ///
    /// // infinities
    /// assert_eq!(f32::INFINITY.rel_error(f32::INFINITY), 0.0);
    /// assert_eq!(0_f32.rel_error(f32::INFINITY), f32::NEG_INFINITY);
    ///
    /// assert_eq!(f32::NEG_INFINITY.rel_error(f32::NEG_INFINITY), 0.0);
    /// assert_eq!(0_f32.rel_error(f32::NEG_INFINITY), f32::NEG_INFINITY);
    ///
    /// // NaNs
    /// assert!(f32::NAN.rel_error(4.0).is_nan());
    /// assert!(4_f32.rel_error(f32::NAN).is_nan());
    /// ```
    ///
    /// Double precision:
    ///
    /// ```rust
    /// use std::f64;
    /// use ieee754::Ieee754;
    ///
    /// assert_eq!(4_f64.rel_error(4.0), 0.0);
    /// assert_eq!(3_f64.rel_error(4.0), -0.25);
    /// assert_eq!(5_f64.rel_error(4.0), 0.25);
    ///
    /// // zero
    /// assert_eq!(0_f64.rel_error(0.0), 0.0);
    /// assert_eq!(1_f64.rel_error(0.0), f64::INFINITY);
    /// assert_eq!((-1_f64).rel_error(0.0), f64::NEG_INFINITY);
    ///
    /// // infinities
    /// assert_eq!(f64::INFINITY.rel_error(f64::INFINITY), 0.0);
    /// assert_eq!(0_f64.rel_error(f64::INFINITY), f64::NEG_INFINITY);
    ///
    /// assert_eq!(f64::NEG_INFINITY.rel_error(f64::NEG_INFINITY), 0.0);
    /// assert_eq!(0_f64.rel_error(f64::NEG_INFINITY), f64::NEG_INFINITY);
    ///
    /// // NaNs
    /// assert!(f64::NAN.rel_error(4.0).is_nan());
    /// assert!(4_f64.rel_error(f64::NAN).is_nan());
    /// ```
    fn rel_error(self, exact: Self) -> Self;
}

macro_rules! mask{
    ($bits: expr; $current: expr => $($other: expr),*) => {
        ($bits >> (0 $(+ $other)*)) & ((1 << $current) - 1)
    }
}
macro_rules! unmask {
    ($x: expr => $($other: expr),*) => {
        $x << (0 $(+ $other)*)
    }
}

#[inline]
#[doc(hidden)]
pub fn abs<F: Ieee754>(x: F) -> F {
    x.abs()
}

macro_rules! mk_impl {
    ($f: ident, $bits: ty, $signed_bits: ty,
     $expn: ty, $expn_raw: ty, $signif: ty,
     $expn_n: expr, $signif_n: expr) => {
        impl Ieee754 for $f {
            type Bits = $bits;
            type Exponent = $expn;
            type RawExponent = $expn_raw;
            type Significand = $signif;
            #[inline]
            fn upto(self, lim: Self) -> Iter<Self> {
                assert!(self <= lim);
                // map -0.0 to 0.0, i.e. ensure that any zero is
                // stored in a canonical manner. This is necessary to
                // use bit-hacks for the comparison in next.
                #[inline(always)]
                fn canon(x: $f) -> $f { if x == 0.0 { 0.0 } else { x } }

                Iter {
                    from: canon(self),
                    to: canon(lim),
                    done: false,
                }
            }
            #[inline]
            fn ulp(self) -> Option<Self> {
                let (_sign, expn, _signif) = self.decompose_raw();

                const MAX_EXPN: $expn_raw = ((1u64 << $expn_n) - 1) as $expn_raw;
                match expn {
                    MAX_EXPN => None,
                    0 => Some($f::recompose_raw(false, 0, 1)),
                    _ => {
                        let ulp_expn = expn.saturating_sub($signif_n);
                        if ulp_expn == 0 {
                            Some($f::recompose_raw(false, 0, 1 << (expn - 1)))
                        } else {
                            Some($f::recompose_raw(false, ulp_expn, 0))
                        }
                    }
                }
            }

            #[inline]
            fn next(self) -> Self {
                let abs_mask = (!(0 as Self::Bits)) >> 1;
                let mut bits = self.bits();
                if self == 0.0 {
                    bits = 1;
                } else if self < 0.0 {
                    bits -= 1;
                    if bits == !abs_mask {
                        // normalise -0.0 to +0.0
                        bits = 0
                    }
                } else {
                    bits += 1
                }
                Ieee754::from_bits(bits)
            }
            #[inline]
            fn prev(self) -> Self {
                let abs_mask = (!(0 as Self::Bits)) >> 1;
                let mut bits = self.bits();
                if self < 0.0 {
                     bits += 1;
                } else if bits & abs_mask == 0 {
                     bits = 1 | !abs_mask;
                } else {
                     bits -= 1;
                }
                Ieee754::from_bits(bits)
            }

            #[inline]
            fn exponent_bias() -> Self::Exponent {
                (1 << ($expn_n - 1)) - 1
            }

            #[inline]
            fn bits(self) -> Self::Bits {
                unsafe {mem::transmute(self)}
            }
            #[inline]
            fn from_bits(bits: Self::Bits) -> Self {
                unsafe {mem::transmute(bits)}
            }
            #[inline]
            fn decompose_raw(self) -> (bool, Self::RawExponent, Self::Significand) {
                let bits = self.bits();

                (mask!(bits; 1 => $expn_n, $signif_n) != 0,
                 mask!(bits; $expn_n => $signif_n) as Self::RawExponent,
                 mask!(bits; $signif_n => ) as Self::Significand)

            }
            #[inline]
            fn recompose_raw(sign: bool, expn: Self::RawExponent, signif: Self::Significand) -> Self {
                Ieee754::from_bits(
                    unmask!(sign as Self::Bits => $expn_n, $signif_n) |
                    unmask!(expn as Self::Bits => $signif_n) |
                    unmask!(signif as Self::Bits => ))
            }

            #[inline]
            fn decompose(self) -> (bool, Self::Exponent, Self::Significand) {
                let (sign, expn, signif) = self.decompose_raw();
                (sign, expn as Self::Exponent - Self::exponent_bias(),
                 signif)
            }
            #[inline]
            fn recompose(sign: bool, expn: Self::Exponent, signif: Self::Significand) -> Self {
                Self::recompose_raw(sign,
                                    (expn + Self::exponent_bias()) as Self::RawExponent,
                                    signif)
            }

            #[inline]
            fn total_cmp(&self, other: &Self) -> Ordering {
                #[inline]
                fn cmp_key(x: $f) -> $signed_bits {
                    let bits = x.bits();
                    let sign_bit = bits & (1 << ($expn_n + $signif_n));
                    let mask = ((sign_bit as $signed_bits) >> ($expn_n + $signif_n)) as $bits >> 1;
                    (bits ^ mask) as $signed_bits
                }
                cmp_key(*self).cmp(&cmp_key(*other))
            }

            #[inline]
            fn abs(self) -> Self {
                let (_, e, s) = self.decompose_raw();
                Self::recompose_raw(false, e, s)
            }

            #[inline]
            fn copy_sign(self, sign: Self) -> Self {
                let (s, _, _) = sign.decompose_raw();
                let (_, e, m) = self.decompose_raw();
                Self::recompose_raw(s, e, m)
            }

            #[inline]
            fn sign(self) -> Self {
                if self == 0.0 || self != self { // is NaN? (.is_nan() added to core in 1.27)
                    self
                } else {
                    1.0.copy_sign(self)
                }
            }

            #[inline]
            fn rel_error(self, exact: Self) -> Self {
                use core::$f;
                if exact == 0.0 {
                    if self == 0.0 {
                        0.0
                    } else {
                        self.sign() * $f::INFINITY
                    }
                } else if abs(exact) == $f::INFINITY {
                    if self == exact {
                        0.0
                    } else if self != self { // is NaN? (.is_nan() added to core in 1.27)
                        self
                    } else {
                        $f::NEG_INFINITY
                    }
                } else {
                    // NaNs propagate
                    (self - exact) / exact
                }
            }
        }

        #[cfg(test)]
        mod $f {
            use std::prelude::v1::*;
            use std::$f;

            use {Ieee754};
            #[test]
            fn upto() {
                assert_eq!((0.0 as $f).upto(0.0).collect::<Vec<_>>(),
                           &[0.0]);
                assert_eq!($f::recompose(false, 1, 1).upto($f::recompose(false, 1, 10)).count(),
                           10);

                assert_eq!($f::recompose(true, -$f::exponent_bias(), 10)
                           .upto($f::recompose(false, -$f::exponent_bias(), 10)).count(),
                           21);
            }
            #[test]
            fn upto_rev() {
                assert_eq!(0.0_f32.upto(0.0_f32).rev().collect::<Vec<_>>(),
                           &[0.0]);

                assert_eq!($f::recompose(false, 1, 1)
                           .upto($f::recompose(false, 1, 10)).rev().count(),
                           10);
                assert_eq!($f::recompose(true, -$f::exponent_bias(), 10)
                           .upto($f::recompose(false, -$f::exponent_bias(), 10)).rev().count(),
                           21);
            }

            #[test]
            fn upto_infinities() {
                use std::$f as f;
                assert_eq!(f::MAX.upto(f::INFINITY).collect::<Vec<_>>(),
                           &[f::MAX, f::INFINITY]);
                assert_eq!(f::NEG_INFINITY.upto(f::MIN).collect::<Vec<_>>(),
                           &[f::NEG_INFINITY, f::MIN]);
            }
            #[test]
            fn upto_infinities_rev() {
                use std::$f as f;
                assert_eq!(f::MAX.upto(f::INFINITY).rev().collect::<Vec<_>>(),
                           &[f::INFINITY, f::MAX]);
                assert_eq!(f::NEG_INFINITY.upto(f::MIN).rev().collect::<Vec<_>>(),
                           &[f::MIN, f::NEG_INFINITY]);
            }

            #[test]
            fn upto_size_hint() {
                let mut iter =
                    $f::recompose(true, -$f::exponent_bias(), 10)
                    .upto($f::recompose(false, -$f::exponent_bias(), 10));

                assert_eq!(iter.size_hint(), (21, Some(21)));
                for i in (0..21).rev() {
                    assert!(iter.next().is_some());
                    assert_eq!(iter.size_hint(), (i, Some(i)));
                }
                assert_eq!(iter.next(), None);
                assert_eq!(iter.size_hint(), (0, Some(0)))
            }

            #[test]
            fn upto_size_hint_rev() {
                let mut iter =
                    $f::recompose(true, -$f::exponent_bias(), 10)
                    .upto($f::recompose(false, -$f::exponent_bias(), 10))
                    .rev();

                assert_eq!(iter.size_hint(), (21, Some(21)));
                for i in (0..21).rev() {
                    assert!(iter.next().is_some());
                    assert_eq!(iter.size_hint(), (i, Some(i)));
                }
                assert_eq!(iter.next(), None);
                assert_eq!(iter.size_hint(), (0, Some(0)))
            }

            #[test]
            fn next_prev_order() {
                let cases = [0.0 as $f, -0.0, 1.0, 1.0001, 1e30, -1.0, -1.0001, -1e30];
                for &x in &cases {
                    assert!(x.next() > x);
                    assert!(x.prev() < x);
                }
            }

            #[test]
            fn ulp_smoke() {
                let smallest_subnormal = $f::recompose_raw(false, 0, 1);
                let smallest_normal = $f::recompose_raw(false, 1, 0);
                assert_eq!((0.0 as $f).ulp(), Some(smallest_subnormal));
                assert_eq!(smallest_subnormal.ulp(), Some(smallest_subnormal));
                assert_eq!($f::recompose_raw(true, 0, 9436).ulp(),
                           Some(smallest_subnormal));
                assert_eq!(smallest_normal.ulp(), Some(smallest_subnormal));

                assert_eq!((1.0 as $f).ulp(),
                           Some($f::recompose(false, -$signif_n, 0)));

                assert_eq!((-123.456e30 as $f).ulp(),
                           Some($f::recompose(false, 106 - $signif_n, 0)));

                assert_eq!($f::INFINITY.ulp(), None);
                assert_eq!($f::NEG_INFINITY.ulp(), None);
                assert_eq!($f::NAN.ulp(), None);
            }

            #[test]
            fn ulp_aggressive() {
                fn check_ulp(x: $f, ulp: $f) {
                    println!("  {:e} {:e}", x, ulp);
                    assert_eq!(x.ulp(), Some(ulp));
                    // with signed-magnitude we need to be moving away
                    let same_sign_ulp = if x < 0.0 { -ulp } else { ulp };

                    assert_ne!(x + same_sign_ulp, x, "adding ulp should be different");

                    if ulp / 2.0 > 0.0 {
                        // floats break ties like this by rounding to
                        // even (in the default mode), so adding half
                        // a ulp may be a new value depending on the
                        // significand.
                        if x.decompose().2 & 1 == 0 {
                            assert_eq!(x + same_sign_ulp / 2.0, x);
                        } else {
                            assert_eq!(x + same_sign_ulp / 2.0, x + same_sign_ulp);
                        }
                    }
                    // no ties to worry about
                    assert_eq!(x + same_sign_ulp / 4.0, x);
                }

                let smallest_subnormal = $f::recompose_raw(false, 0, 1);
                let mut ulp = smallest_subnormal;

                check_ulp(0.0, ulp);

                let mut pow2 = smallest_subnormal;
                for i in 0..200 {
                    println!("{}", i);
                    check_ulp(pow2, ulp);
                    check_ulp(-pow2, ulp);

                    let (_, e, _) = pow2.decompose_raw();
                    if e > 0 {
                        for &signif in &[1,
                                         // random numbers
                                         9436, 1577069,
                                         // last two for this exponent
                                         (1 << $signif_n) - 2, (1 << $signif_n) - 1] {
                            check_ulp($f::recompose_raw(false, e, signif), ulp);
                            check_ulp($f::recompose_raw(true, e, signif), ulp);
                        }
                    }

                    pow2 *= 2.0;
                    if i >= $signif_n {
                        ulp *= 2.0;
                    }
                }
            }

            #[test]
            fn abs() {
                let this_abs = <$f as Ieee754>::abs;
                assert!(this_abs($f::NAN).is_nan());

                let cases = [0.0 as $f, -1.0, 1.0001,
                             // denormals
                             $f::recompose_raw(false, 0, 123), $f::recompose(true, 0, 123),
                             $f::NEG_INFINITY, $f::INFINITY];
                for x in &cases {
                    assert_eq!(this_abs(*x), x.abs());
                }
            }

            #[test]
            fn total_cmp() {
                let nan_exp = $f::NAN.decompose_raw().1;
                let q = 1 << ($signif_n - 1);

                let qnan0 = $f::recompose_raw(false, nan_exp, q);
                let qnan1 = $f::recompose_raw(false, nan_exp, q | 1);
                let qnanlarge = $f::recompose_raw(false, nan_exp, q | (q - 1));

                let snan1 = $f::recompose_raw(false, nan_exp, 1);
                let snan2 = $f::recompose_raw(false, nan_exp, 2);
                let snanlarge = $f::recompose_raw(false, nan_exp, q - 1);

                let subnormal = $f::recompose_raw(false, 0, 1);

                // it's a total order, so we can literally write
                // options in order, and compare them all, using their
                // indices as ground-truth. NB. the snans seem to
                // get canonicalized to qnan on some versions of i686
                // Linux (using `cross` on Travis CI), so we can't
                // include them.
                let include_snan = cfg!(not(target_arch = "x86"));
                // -qNaN
                let mut cases = vec![-qnanlarge, -qnan1, -qnan0];
                // -sNaN
                if include_snan {
                    cases.extend_from_slice(&[-snanlarge, -snan2, -snan1]);
                }
                // Numbers (note -0, +0)
                cases.extend_from_slice(&[
                    $f::NEG_INFINITY,
                    -1e15, -1.001, -1.0, -0.999, -1e-15, -subnormal,
                    -0.0, 0.0,
                    subnormal, 1e-15, 0.999, 1.0, 1.001, 1e15,
                    $f::INFINITY
                ]);
                // +sNaN
                if include_snan {
                    cases.extend_from_slice(&[snan1, snan2, snanlarge]);
                }
                // +qNaN
                cases.extend_from_slice(&[qnan0, qnan1, qnanlarge]);

                for (ix, &x) in cases.iter().enumerate() {
                    for (iy, &y) in cases.iter().enumerate() {
                        let computed = x.total_cmp(&y);
                        let expected = ix.cmp(&iy);
                        assert_eq!(
                            computed, expected,
                            "{:e} ({}, {:?}) cmp {:e} ({}, {:?}), got: {:?}, expected: {:?}",
                            x, ix, x.decompose(),
                            y, iy, y.decompose(),
                            computed, expected);
                    }
                }
            }

            #[test]
            fn copy_sign() {
                let positives = [$f::NAN, $f::INFINITY, 1e10, 1.0, 0.0];

                for &pos in &positives {
                    assert_eq!((1 as $f).copy_sign(pos), 1.0);
                    assert_eq!((1 as $f).copy_sign(-pos), -1.0);
                    assert_eq!((-1 as $f).copy_sign(pos), 1.0);
                    assert_eq!((-1 as $f).copy_sign(-pos), -1.0);

                    assert_eq!($f::INFINITY.copy_sign(pos), $f::INFINITY);
                    assert_eq!($f::INFINITY.copy_sign(-pos), $f::NEG_INFINITY);
                    assert_eq!($f::NEG_INFINITY.copy_sign(pos), $f::INFINITY);
                    assert_eq!($f::NEG_INFINITY.copy_sign(-pos), $f::NEG_INFINITY);

                    assert!($f::NAN.copy_sign(pos).is_nan());
                    assert!($f::NAN.copy_sign(-pos).is_nan());
                }
            }

            #[test]
            fn sign() {
                assert!($f::NAN.sign().is_nan());

                assert_eq!($f::NEG_INFINITY.sign(), -1.0);
                assert_eq!((-1e10 as $f).sign(), -1.0);
                assert_eq!((-1.0 as $f).sign(), -1.0);
                assert_eq!((-0.0 as $f).sign().bits(), (-0.0 as $f).bits());
                assert_eq!((0.0 as $f).sign().bits(), (0.0 as $f).bits());
                assert_eq!((1.0 as $f).sign(), 1.0);
                assert_eq!((1e10 as $f).sign(), 1.0);
                assert_eq!($f::INFINITY.sign(), 1.0);
            }

            #[test]
            fn rel_error() {
                let zer: $f = 0.0;
                let one: $f = 1.0;
                let two: $f = 2.0;

                assert_eq!(zer.rel_error(one), -1.0);
                assert_eq!(one.rel_error(one), 0.0);
                assert_eq!((-one).rel_error(one), -2.0);
                assert_eq!(two.rel_error(one), 1.0);
                assert_eq!((-two).rel_error(one), -3.0);

                assert_eq!(zer.rel_error(-one), -1.0);
                assert_eq!(one.rel_error(-one), -2.0);
                assert_eq!((-one).rel_error(-one), 0.0);
                assert_eq!(two.rel_error(-one), -3.0);
                assert_eq!((-two).rel_error(-one), 1.0);

                assert_eq!(zer.rel_error(two), -1.0);
                assert_eq!(one.rel_error(two), -0.5);
                assert_eq!((-one).rel_error(two), -1.5);
                assert_eq!(two.rel_error(two), 0.0);
                assert_eq!((-two).rel_error(two), -2.0);

                assert_eq!(zer.rel_error(-two), -1.0);
                assert_eq!(one.rel_error(-two), -1.5);
                assert_eq!((-one).rel_error(-two), -0.5);
                assert_eq!(two.rel_error(-two), -2.0);
                assert_eq!((-two).rel_error(-two), 0.0);
            }
            #[test]
            fn rel_error_edge_cases() {
                let nan = $f::NAN;
                let inf = $f::INFINITY;
                let zer: $f = 0.0;
                let one: $f = 1.0;

                assert!(nan.rel_error(nan).is_nan());
                assert!(zer.rel_error(nan).is_nan());
                assert!(nan.rel_error(zer).is_nan());

                assert_eq!(zer.rel_error(zer), 0.0);
                assert_eq!(zer.rel_error(-zer), 0.0);
                assert_eq!((-zer).rel_error(zer), 0.0);
                assert_eq!((-zer).rel_error(-zer), 0.0);
                assert_eq!(one.rel_error(zer), inf);
                assert_eq!((-one).rel_error(zer), -inf);
                assert_eq!(inf.rel_error(zer), inf);
                assert_eq!((-inf).rel_error(zer), -inf);


                assert_eq!(inf.rel_error(inf), 0.0);
                assert_eq!(inf.rel_error(-inf), -inf);
                assert_eq!((-inf).rel_error(inf), -inf);
                assert_eq!((-inf).rel_error(-inf), 0.0);
                assert_eq!(zer.rel_error(inf), -inf);
                assert_eq!(zer.rel_error(-inf), -inf);
            }
        }
    }
}

mk_impl!(f32, u32, i32, i16, u8, u32, 8, 23);
mk_impl!(f64, u64, i64, i16, u16, u64, 11, 52);
