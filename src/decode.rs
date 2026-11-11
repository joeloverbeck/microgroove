//! Decode: a file path in; the whole file as samples at its native rate, or a plain refusal.

use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;

use md5::{Digest, Md5};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// A whole audio file in memory. Mono files hold one channel, stereo files two.
pub struct Song {
    pub sample_rate: u32,
    pub channels: Vec<Vec<f32>>,
}

impl Song {
    pub fn frames(&self) -> usize {
        self.channels[0].len()
    }

    pub fn left(&self) -> &[f32] {
        &self.channels[0]
    }

    /// A mono file sounds the same on both sides.
    pub fn right(&self) -> &[f32] {
        &self.channels[self.channels.len() - 1]
    }

    pub fn duration_s(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// MD5 of the audio as interleaved 24-bit little-endian PCM: the identity that
    /// `ffmpeg -i <file> -map 0:a -c:a pcm_s24le -f md5 -` prints.
    pub fn pcm24_md5(&self) -> String {
        let mut hasher = Md5::new();
        let mut block = Vec::with_capacity(3 * self.channels.len() * 4096);
        for start in (0..self.frames()).step_by(4096) {
            block.clear();
            for frame in start..(start + 4096).min(self.frames()) {
                for channel in &self.channels {
                    let value = (f64::from(channel[frame]) * 8_388_608.0).round();
                    let value = value.clamp(-8_388_608.0, 8_388_607.0) as i32;
                    block.extend_from_slice(&value.to_le_bytes()[..3]);
                }
            }
            hasher.update(&block);
        }
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

const UNSUPPORTED: &str = "it is not a supported audio format (FLAC, WAV or Ogg Vorbis)";
/// The sample rates analysis can handle. Its loudness meter weighs the sound with a
/// filter centred near 1,682 Hz, which takes a sample rate of more than twice that, and
/// accepts none above 2,822,400 Hz. (Below 1,650 Hz the spectrum's bands do not fit either.)
const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 3_364..=2_822_400;

/// The whole file, or why it was not accepted, in one plain line. Nothing is ever
/// substituted for a refused file.
pub fn decode(path: &Path) -> Result<Song, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err("the file does not exist".into());
        }
        Err(error) => return Err(format!("the file cannot be read ({error})")),
    };
    if file.metadata().is_ok_and(|metadata| metadata.is_dir()) {
        return Err("it is a directory, not an audio file".into());
    }

    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let Ok(mut format) = symphonia::default::get_probe().probe(
        &Hint::new(),
        stream,
        FormatOptions::default(),
        MetadataOptions::default(),
    ) else {
        return Err(UNSUPPORTED.into());
    };
    let Some(track) = format.default_track(TrackType::Audio) else {
        return Err(UNSUPPORTED.into());
    };
    let track_id = track.id;
    let Some(Ok(mut decoder)) = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .map(|params| {
            symphonia::default::get_codecs()
                .make_audio_decoder(params, &AudioDecoderOptions::default())
        })
    else {
        return Err(UNSUPPORTED.into());
    };

    let mut sample_rate = 0;
    let mut channels: Vec<Vec<f32>> = Vec::new();
    let mut interleaved: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => return Err(format!("the audio cannot be decoded ({error})")),
        };
        if packet.track_id != track_id {
            continue;
        }
        let buffer = match decoder.decode(&packet) {
            Ok(buffer) => buffer,
            Err(error) => return Err(format!("the audio cannot be decoded ({error})")),
        };
        let (rate, count) = (buffer.spec().rate(), buffer.spec().channels().count());
        if channels.is_empty() {
            if count == 0 || count > 2 {
                return Err(format!(
                    "it has {count} channels; only mono and stereo are supported"
                ));
            }
            if !SAMPLE_RATES.contains(&rate) {
                let (least, most) = SAMPLE_RATES.into_inner();
                return Err(format!(
                    "its sample rate is {rate} Hz; analysis handles {least} Hz to {most} Hz"
                ));
            }
            sample_rate = rate;
            channels = vec![Vec::new(); count];
        } else if rate != sample_rate || count != channels.len() {
            return Err("its sample rate or channel count changes part-way through".into());
        }
        buffer.copy_to_vec_interleaved(&mut interleaved);
        for frame in interleaved.chunks_exact(count) {
            for (channel, sample) in channels.iter_mut().zip(frame) {
                channel.push(*sample);
            }
        }
    }
    if channels.first().is_none_or(|channel| channel.is_empty()) {
        return Err("it holds no audio".into());
    }
    Ok(Song {
        sample_rate,
        channels,
    })
}
