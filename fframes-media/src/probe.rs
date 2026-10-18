use crate::FFramesMediaError;
use crate::error::Result;
use ffmpeg_sys_fframes::*;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

/// What a media file contains, read from its container without decoding.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaStreams {
    /// The container format's short names, e.g. `mov,mp4,m4a,3gp,3g2,mj2` or `matroska,webm`.
    pub format: String,
    /// Codec name of the best video stream (`h264`, `hevc`, `vp9`, `mpeg4`, ...).
    pub video_codec: Option<String>,
    pub width: u32,
    pub height: u32,
    /// Average frame rate of the video stream; 0 when the container does not say.
    pub fps: f64,
    /// Codec name of the best audio stream (`aac`, `opus`, ...).
    pub audio_codec: Option<String>,
}

/// Whether the linked `FFmpeg` has an encoder with this name (`libx264`, `mpeg4`, ...).
pub fn has_encoder(name: &str) -> bool {
    let Ok(name) = CString::new(name) else {
        return false;
    };
    unsafe { !avcodec_find_encoder_by_name(name.as_ptr()).is_null() }
}

/// Reads the streams of a media file.
pub fn probe_streams(path: &Path) -> Result<MediaStreams> {
    let path = CString::new(path.to_string_lossy().as_ref())?;
    unsafe {
        let mut fmt_ctx: *mut AVFormatContext = ptr::null_mut();
        let ret = avformat_open_input(
            &raw mut fmt_ctx,
            path.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        if ret < 0 {
            return Err(FFramesMediaError::LibAVAudioDecodingError((
                ret,
                "Could not open input file".to_string(),
            )));
        }
        let ret = avformat_find_stream_info(fmt_ctx, ptr::null_mut());
        if ret < 0 {
            avformat_close_input(&raw mut fmt_ctx);
            return Err(FFramesMediaError::LibAVAudioDecodingError((
                ret,
                "Could not find stream information".to_string(),
            )));
        }
        let name = |ptr: *const std::ffi::c_char| {
            (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_string_lossy().into_owned())
        };
        let stream = |kind| {
            let index = av_find_best_stream(fmt_ctx, kind, -1, -1, ptr::null_mut(), 0);
            (index >= 0).then(|| *(*fmt_ctx).streams.add(index as usize))
        };
        let mut streams = MediaStreams {
            format: name((*(*fmt_ctx).iformat).name).unwrap_or_default(),
            video_codec: None,
            width: 0,
            height: 0,
            fps: 0.,
            audio_codec: None,
        };
        if let Some(video) = stream(AVMediaType::AVMEDIA_TYPE_VIDEO) {
            let parameters = (*video).codecpar;
            streams.video_codec = name(avcodec_get_name((*parameters).codec_id));
            streams.width = (*parameters).width.max(0) as u32;
            streams.height = (*parameters).height.max(0) as u32;
            let rate = (*video).avg_frame_rate;
            if rate.num > 0 && rate.den > 0 {
                streams.fps = f64::from(rate.num) / f64::from(rate.den);
            }
        }
        if let Some(audio) = stream(AVMediaType::AVMEDIA_TYPE_AUDIO) {
            streams.audio_codec = name(avcodec_get_name((*(*audio).codecpar).codec_id));
        }
        avformat_close_input(&raw mut fmt_ctx);
        Ok(streams)
    }
}
