//! The diagnostic path of [`Albedo::reflect`]: the same radiance, with the reason for a
//! refusal (flyingrobots/bunny#445).

use core::fmt;

use bunny_num::FixedQ32_32;

use super::Albedo;

/// Why [`Albedo::try_reflect`] refused its radiance.
///
/// [`Albedo::reflect`] returns `None` exactly when `try_reflect` returns an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlbedoReflectError {
    /// `direct + indirect` of one channel is outside Q32.32: above raw `i64::MAX` or below raw
    /// `i64::MIN`.
    SumUnrepresentable {
        /// The first such channel: `0`, `1` or `2`.
        channel: usize,
        /// That channel of `direct`, as given.
        direct: FixedQ32_32,
        /// That channel of `indirect`, as given.
        indirect: FixedQ32_32,
    },
}

impl Albedo {
    /// The diagnostic path of [`Self::reflect`]: the same radiance, with the reason for a
    /// refusal. `OPTICS-TP-336` checks over generated radiance that `reflect` returns
    /// `Some(r)` exactly when this returns `Ok(r)`.
    ///
    /// # Errors
    ///
    /// [`AlbedoReflectError::SumUnrepresentable`] naming the first channel, in order, whose
    /// exact sum `direct_c + indirect_c` is outside raw `i64::MIN..=i64::MAX`.
    pub fn try_reflect(
        &self,
        direct: [FixedQ32_32; 3],
        indirect: [FixedQ32_32; 3],
    ) -> Result<[FixedQ32_32; 3], AlbedoReflectError> {
        let mut reflected = [FixedQ32_32::ZERO; 3];
        let channels = self.channels().into_iter().zip(direct.into_iter().zip(indirect));
        for (channel, (albedo, (d, i))) in channels.enumerate() {
            let sum = d.checked_add(i).ok_or(AlbedoReflectError::SumUnrepresentable {
                channel,
                direct: d,
                indirect: i,
            })?;
            if let Some(slot) = reflected.get_mut(channel) {
                *slot = albedo * sum;
            }
        }
        Ok(reflected)
    }
}

impl fmt::Display for AlbedoReflectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::SumUnrepresentable { channel, direct, indirect } = self;
        let (direct, indirect) = (direct.raw(), indirect.raw());
        write!(
            f,
            "radiance sum of channel {channel}, raw {direct} + {indirect}, is not representable \
             in Q32.32"
        )
    }
}

impl core::error::Error for AlbedoReflectError {}
