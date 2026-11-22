//! Local image text recognition: native Windows OCR or an explicit Tesseract provider.
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use std::path::Path;
pub fn validate_language(language: &str) -> CoreResult<()> {
    if language.is_empty()
        || language.len() > 64
        || !language.as_bytes()[0].is_ascii_alphanumeric()
        || !language
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || b"_+-".contains(&value))
    {
        return Err(CoreError::InvalidInput(
            "enter a language code such as en-US (Windows) or eng+deu (Tesseract)".into(),
        ));
    }
    Ok(())
}
fn tesseract_arguments(input: &Path, language: &str) -> CoreResult<Vec<String>> {
    Ok(vec![
        super::media_provider::provider_path_text(input)?,
        "stdout".into(),
        "-l".into(),
        language.into(),
        "--psm".into(),
        "3".into(),
    ])
}
pub fn recognize(
    path: &Path,
    provider: &str,
    language: &str,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<String> {
    validate_language(language)?;
    cancel.check()?;
    let image = super::resize::decode_oriented(path)?;
    progress(Progress {
        done: 0,
        total: 1,
        message: "Recognizing image text locally".into(),
    });
    let result = match provider {
        "native" => native(&image, language, cancel)?,
        "tesseract" => {
            let mut folder = super::media_provider::TemporaryFolder::new(&std::env::temp_dir())?;
            let input = folder.file("ocr.png")?;
            let args = tesseract_arguments(&input, language)?;
            image
                .save_with_format(&input, image::ImageFormat::Png)
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            let executable = super::media_provider::find_executable("tesseract")?;
            super::media_provider::run_text(
                &executable,
                &args,
                cancel,
                std::time::Duration::from_secs(300),
                1024 * 1024,
            )?
        }
        _ => {
            return Err(CoreError::InvalidInput(
                "choose Windows OCR or Tesseract".into(),
            ));
        }
    };
    cancel.check()?;
    progress(Progress {
        done: 1,
        total: 1,
        message: "Text recognition complete".into(),
    });
    Ok(result)
}
#[cfg(target_os = "windows")]
fn native(image: &image::DynamicImage, language: &str, cancel: &CancelToken) -> CoreResult<String> {
    use windows::{
        Globalization::Language,
        Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::DataWriter,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::HSTRING,
    };
    let error = |error: windows::core::Error| {
        CoreError::InvalidInput(format!(
            "Windows OCR failed; check that the requested OCR language is installed: {error}"
        ))
    };
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    RoUninitialize();
                }
            }
        }
    }
    let _apartment = match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => Apartment(true),
        Err(value) if value.code().0 == 0x80010106_u32 as i32 => Apartment(false),
        Err(value) => return Err(error(value)),
    };
    let maximum = OcrEngine::MaxImageDimension().map_err(error)?;
    let image = image.thumbnail(maximum, maximum).to_luma8();
    let writer = DataWriter::new().map_err(error)?;
    writer.WriteBytes(image.as_raw()).map_err(error)?;
    let buffer = writer.DetachBuffer().map_err(error)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Gray8,
        image.width() as i32,
        image.height() as i32,
    )
    .map_err(error)?;
    let language = Language::CreateLanguage(&HSTRING::from(language)).map_err(error)?;
    let engine = OcrEngine::TryCreateFromLanguage(&language).map_err(error)?;
    let operation = engine.RecognizeAsync(&bitmap).map_err(error)?;
    let started = std::time::Instant::now();
    while operation.Status().map_err(error)?.0 == 0 {
        if let Err(cancelled) = cancel.check() {
            let _ = operation.Cancel();
            return Err(cancelled);
        }
        if started.elapsed() > std::time::Duration::from_secs(300) {
            let _ = operation.Cancel();
            return Err(CoreError::InvalidInput(
                "Windows OCR exceeded its five-minute limit".into(),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let result = operation.GetResults().map_err(error)?;
    let text = result.Text().map_err(error)?.to_string();
    if text.len() > 1024 * 1024 {
        return Err(CoreError::InvalidInput(
            "recognized text exceeds 1 MiB".into(),
        ));
    }
    Ok(text)
}
#[cfg(not(target_os = "windows"))]
fn native(
    _image: &image::DynamicImage,
    _language: &str,
    _cancel: &CancelToken,
) -> CoreResult<String> {
    Err(CoreError::Unsupported(
        "this OS native OCR (choose Tesseract)",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_language_paths_and_options() {
        assert!(validate_language("eng+deu").is_ok());
        for invalid in ["../secret", "--help", "", "eng/file"] {
            assert!(validate_language(invalid).is_err());
        }
    }

    #[test]
    fn verification56_tesseract_temporary_argument_refuses_ambiguous_path_without_launching() {
        assert_eq!(
            tesseract_arguments(Path::new("generated temporary folder /ocr.png"), "eng").unwrap()
                [0],
            "generated temporary folder /ocr.png"
        );
        assert_eq!(
            tesseract_arguments(Path::new("generated\nfolder/ocr.png"), "eng")
                .unwrap_err()
                .code(),
            "invalid_input"
        );
        #[cfg(unix)]
        let invalid = {
            use std::os::unix::ffi::OsStringExt;
            std::path::PathBuf::from(std::ffi::OsString::from_vec(
                b"generated-\xff/ocr.png".to_vec(),
            ))
        };
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            std::path::PathBuf::from(std::ffi::OsString::from_wide(&[
                0x0061, 0xd800, 0x002f, 0x006f, 0x0063, 0x0072, 0x002e, 0x0070, 0x006e, 0x0067,
            ]))
        };
        #[cfg(any(unix, windows))]
        assert_eq!(
            tesseract_arguments(&invalid, "eng").unwrap_err().code(),
            "invalid_input"
        );
    }
}
