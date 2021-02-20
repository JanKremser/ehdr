use std::path::Path;
use std::process::{Command};
use std::thread;
use std::sync::mpsc;

#[cfg(windows)]
const NEW_LINE: &'static str = "\r\n";
#[cfg(not(windows))]
const NEW_LINE: &'static str = "\n";

#[derive(Debug)]
pub struct Video<'a> {
    file: &'a Path,
    meta: serde_json::Value,
    height: u64,
    width: u64,
    crop_x: u64,
    crop_y: u64,
}

impl <'a> Video<'a> {
    pub fn new(file: &'a Path) -> Option<Self> {
        let meta = Self::read_metadata(
            file.to_str()?
        )?;

        let height = meta["frames"][0]["width"].as_u64()?;
        let width = meta["frames"][0]["height"].as_u64()?;

        Some(
            Video {
                file,
                meta,
                height,
                width,
                crop_x: 0,
                crop_y: 0,
            }
        )
    }

    pub fn get_path_str(&self) -> Option<&str>{
        Some(
            self.file.to_str()?
        )
    }

    pub fn get_pix_fmt(&self) -> Option<&str> {
        let param_str = self.meta["frames"][0]["pix_fmt"].as_str()?;
        Some(
            Self::format_meta_str(param_str)?
        )
    }

    pub fn get_color_primaries(&self) -> Option<&str> {
        let param_str = self.meta["frames"][0]["color_primaries"].as_str()?;
        Some(
            Self::format_meta_str(param_str)?
        )
    }

    pub fn get_color_space(&self) -> Option<&str> {
        let param_str = self.meta["frames"][0]["color_space"].as_str()?;
        Some(
            Self::format_meta_str(param_str)?
        )
    }

    pub fn get_color_transfer(&self) -> Option<&str> {
        let param_str = self.meta["frames"][0]["color_transfer"].as_str()?;
        Some(
            Self::format_meta_str(param_str)?
        )
    }

    pub fn get_side_data_list_param(&self, param: &str) -> Option<&str> {
        let param_str = self.meta["frames"][0]["side_data_list"][0][param].as_str()?;
        Some(
            Self::format_meta_str(param_str)?
        )
    }

    pub fn get_width(&self) -> u64 {
        self.width
    }

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn get_ffmpeg_crop_str(&self) -> String {
        format!("crop={w}:{h}:{x}:{y}",
            w = self.width,
            h = self.height,
            x = self.crop_x,
            y = self.crop_y,
        )
    }

    pub fn is_hdr_video(&self) -> bool {
        match self.get_pix_fmt() {
            Some("yuv420p10le") => true,
            _ => false,
        }
    }

    pub fn crop_video(&mut self) {
        println!("\n\n====> Start Check-Cropfactor");

        let start_sec = 60;
        let threads_count = 10;

        let time = self.meta["format"]["duration"].as_str().unwrap();
        let time_sec = time.split(".").next().unwrap().parse::<u64>().unwrap() - start_sec;

        let time_segment = time_sec / threads_count;

        let (tx, rx) = mpsc::channel();

        let mut count = 0;
        while threads_count > count {
            let path = format!("{}", self.get_path_str().unwrap());
            let s_sec = start_sec + (time_segment * count);

            // clone the sender
            let tx_clone = mpsc::Sender::clone(&tx);

            thread::spawn(move || {
                println!("start crop scan: {}sec", s_sec);
                tx_clone.send(
                    read_crop(&path, s_sec)
                ).unwrap();
            });

            count += 1;
        }

        let mut crop: VideoCrop = VideoCrop::new_clean();
        let mut close_threads = 0;

        for vc in rx.iter() {
            if crop.is_smaller_than(&vc) {
                crop = vc;
            }

            close_threads += 1;
            if close_threads == threads_count {
                self.crop_x = crop.x;
                self.crop_y = crop.y;
                self.width = crop.w;
                self.height = crop.h;
                println!("final crop: {:?}", crop);
                return;
            }
        }

    }

    fn format_meta_str(param_str: &str) -> Option<&str> {
        let re = regex::Regex::new(r"/").ok()?;
        let fields: Vec<&str> = re.splitn(param_str, 2).collect();

        Some(
            fields[0]
        )
    }

    fn read_metadata(input_file: &str) -> Option<serde_json::Value> {
        let output = Command::new("ffprobe")
            .args(&[
                "-hide_banner", 
                "-loglevel", "warning",
                "-select_streams", "v",
                "-print_format", "json",
                "-show_frames",
                "-read_intervals", "%+#1",
                "-show_entries", "frame=color_space,color_primaries,color_transfer,side_data_list,pix_fmt,width,height",
                "-show_entries", "format=duration",
                "-i", input_file
            ])
            .output()
            .expect("ffprobe command failed to start");
        
        let out = String::from_utf8(output.stdout).expect("Failed to convert stdout to string.");
    
        Some(
            serde_json::from_str(&out).ok()?
        )
    }
}


fn read_crop(input_file: &str, start_sec: u64) -> VideoCrop {
    let output = Command::new("ffmpeg")
        .args(&[
            "-ss", &format!("{}", start_sec),//starttime
            "-t", "60",//offset 60 sec
            "-i", input_file,
            "-vf", "cropdetect",
            "-f", "null", "-",
        ])
        .output()
        .expect("ffmpeg command failed to start");

    let out = String::from_utf8(output.stderr).expect("Failed to convert stdout to string.");

    let re = regex::Regex::new(r"crop=").unwrap();
    let fields: Vec<&str> = re.split(&out).collect();

    let mut crop: VideoCrop = VideoCrop::new_clean();

    for i in fields {
        let crop_temp = VideoCrop::new(
            &i.split(NEW_LINE).next().unwrap()
        );
        
        if let Some(s_corp) = crop_temp {
            if crop.is_smaller_than(&s_corp) {
                crop = s_corp;
            }
        }
    }

    println!("crop: {:?}\n", crop);

    crop
}

#[derive(Debug)]
struct VideoCrop {
    pub w: u64,
    pub h: u64,
    pub x: u64,
    pub y: u64
}

impl VideoCrop {
    fn new(crop: &str) -> Option<Self> {
        let mut split = crop.split(":");
        Some(
            VideoCrop {
                w: split.next()?.parse::<u64>().ok()?,
                h: split.next()?.parse::<u64>().ok()?,
                x: split.next()?.parse::<u64>().ok()?,
                y: split.next()?.parse::<u64>().ok()?,
            }
        )
    }
    fn new_clean() -> Self{
        VideoCrop {
            w: 0,
            h: 0,
            x: 0,
            y: 0,
        }
    }
    fn is_smaller_than(&self, crop: &VideoCrop) -> bool {
        !(self.w > crop.w && self.h > crop.h && self.x <= crop.x && self.y <= crop.y)
    }
}