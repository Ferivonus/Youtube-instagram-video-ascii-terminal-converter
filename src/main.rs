use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};

use image::GenericImageView;
use rodio::{Decoder, OutputStream, Sink};
use walkdir::WalkDir;

const VIDEO_FILE: &str = "input.mp4";
const AUDIO_FILE: &str = "output.mp3";
const FRAME_DIR: &str = "frames";

// Adjust resolution for smoother but smaller display (e.g., 100x40)
const SCALE_WIDTH: u32 = 140;
const SCALE_HEIGHT: u32 = 52;

// 30 FPS for smoother video
const FPS: u64 = 30;
const FRAME_DURATION_MS: u64 = 1000 / FPS;

const ASCII_CHARS: &[u8] =
    b"$@B%8&WM#*oahkbdpqwmZ0OQLCJUYXzcvunxrjft/|()1{}[]?-_+~<>i!lI;:,\"^`'. ";

fn download_youtube_video(video_url: &str) {
    println!("[*] YouTube videosu indiriliyor...");
    let status = Command::new("./yt-dlp.exe")
        .args([
            "-f",
            "bestvideo[ext=mp4]+bestaudio[ext=m4a]/mp4",
            "-o",
            VIDEO_FILE,
            video_url,
        ])
        .status()
        .expect("./yt-dlp.exe çalıştırılamadı");

    if !status.success() {
        panic!("YouTube video indirme başarısız.");
    }
    println!("[+] Video indirildi.");
}

fn extract_audio_and_frames() {
    if Path::new("use_that_one.mp3").exists() {
        println!("[*] 'use_that_one.mp3' bulundu. Ses dönüştürülmeyecek.");
        fs::copy("use_that_one.mp3", AUDIO_FILE).expect("use_that_one.mp3 kopyalanamadı.");
    } else {
        println!("[*] Ses çıkartılıyor...");
        Command::new("./ffmpeg.exe")
            .args(["-i", VIDEO_FILE, "-q:a", "0", "-map", "a", AUDIO_FILE, "-y"])
            .status()
            .expect("./ffmpeg ses çıkarma başarısız.");
    }

    println!("[*] Kareler çıkartılıyor...");
    fs::create_dir_all(FRAME_DIR).unwrap();

    Command::new("./ffmpeg.exe")
        .args([
            "-i",
            VIDEO_FILE,
            "-vf",
            &format!("scale={}x{}", SCALE_WIDTH, SCALE_HEIGHT),
            "-r",
            &FPS.to_string(),
            &format!("{}/frame_%04d.png", FRAME_DIR),
            "-y",
        ])
        .status()
        .expect("./ffmpeg kare çıkarma başarısız.");
}

fn image_to_ascii(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    let mut ascii = String::new();

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let norm = luma / 255.0;
            let idx = (norm * (ASCII_CHARS.len() - 1) as f32).round() as usize;
            ascii.push(ASCII_CHARS[idx] as char);
        }
        ascii.push('\n');
    }

    Ok(ascii)
}

fn play_audio() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let file = BufReader::new(File::open(AUDIO_FILE).unwrap());
    let source = Decoder::new(file).unwrap();
    sink.append(source);
    sink.sleep_until_end();
}

fn play_ascii_video() {
    let mut frame_paths: Vec<_> = WalkDir::new(FRAME_DIR)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"))
        .map(|e| e.path().to_owned())
        .collect();

    frame_paths.sort();

    let start = Instant::now();

    for (i, path) in frame_paths.iter().enumerate() {
        let target_time = Duration::from_millis(i as u64 * FRAME_DURATION_MS);
        let now = Instant::now();
        if target_time > now.duration_since(start) {
            thread::sleep(target_time - now.duration_since(start));
        }

        if let Ok(ascii) = image_to_ascii(path.to_str().unwrap()) {
            execute!(std::io::stdout(), Clear(ClearType::All)).unwrap();
            println!("{}", ascii);
        }
    }
}

fn get_video_url_from_user() -> String {
    println!("Lütfen YouTube video linkini girin:");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Girdi alınamadı.");
    input.trim().to_string()
}

fn main() {
    let video_url = get_video_url_from_user();

    if !Path::new(VIDEO_FILE).exists() {
        download_youtube_video(&video_url);
    }

    extract_audio_and_frames();

    let audio_thread = thread::spawn(play_audio);
    play_ascii_video();
    audio_thread.join().unwrap();
}
