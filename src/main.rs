use std::string::String;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::fs;

use clap::{Arg, App};

#[cfg(windows)]
const NEW_LINE: &'static str = "\r\n";
#[cfg(not(windows))]
const NEW_LINE: &'static str = "\n";

fn main() {
    let matches = App::new("SimpleConvert")
        .version("1.1")
        .author("Jan Kremser")
        .about("[...]")
        .arg(Arg::with_name("input")
            .short("i")
            .long("input")
            .value_name("FILE/FOLDER")
            .help("input file or folder")
            .required(true)
        ).arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE/FOLDER")
            .help("output file or folder")
            .required(true)
        ).arg(Arg::with_name("crf")
            .long("crf")
            .value_name("INT")
            .help("crf from ffmpeg (default is [13; >=UHD] to [18, =FHD] && [20; >FHD] )")
        ).arg(Arg::with_name("preset")
            .short("p")
            .long("preset")
            .value_name("STRING")
            .help("preset from ffmpeg (default is 'auto', alternative: [ultrafast, superfast, veryfast, faster, fast, medium])")
        ).arg(Arg::with_name("crop")
            .short("c")
            .long("crop")
            .help("Auto crop function")
        ).arg(Arg::with_name("hdr")
            .short("H")
            .long("hdr")
            .help("copy hdr metadata")
        ).get_matches();

    let mut crf: u8 = 0;
    if matches.is_present("crf") {
        crf = matches.value_of("crf").unwrap().parse::<u8>().unwrap();
    }

    let mut preset: &str = "";
    if matches.is_present("preset") {
        preset = matches.value_of("preset").unwrap();
    }

    let path_input = Path::new(matches.value_of("input").unwrap());

    if path_input.is_dir() {
        let path_output = Path::new(matches.value_of("output").unwrap());
        if !path_output.is_dir() {
            panic!("output is not a folder");
        }

        for entry in fs::read_dir(path_input).unwrap() {
            let entry = entry.unwrap();
            let input_file = entry.path();
            if input_file.is_file() {
                let mut output_file = path_output.to_path_buf();
                output_file.push(input_file.file_name().unwrap());

                match input_file.extension().unwrap().to_str().unwrap().to_uppercase().as_str() {
                    "MKV" | "MP4" => {
                        println!("input:   {:?}", input_file.to_str().unwrap());
                        println!("output:  {:?}", output_file.to_str().unwrap());

                        convert(
                            input_file.to_str().unwrap(), 
                            output_file.to_str().unwrap(),
                            &matches.is_present("crop"),
                            &crf,
                            preset,
                            &matches.is_present("hdr"),
                        );
                    }
                    _ => print!("file is not suportet")
                }
            }
        }
    } else {
        convert(
            matches.value_of("input").unwrap(), 
            matches.value_of("output").unwrap(),
            &matches.is_present("crop"),
            &crf,
            preset,
            &matches.is_present("hdr"),
        );
    }
}

fn convert(input_file: &str, output_file: &str, is_crop_active: &bool, crf: &u8, preset: &str, is_hdr: &bool) {
    let v: serde_json::Value = read_hdr_metadata(input_file);
    println!("---Metadata: \n{:#?}\n\n", v);

    let mut ffmpeg: Command = Command::new("ffmpeg");
    ffmpeg.args(&[
        "-i", input_file,
        "-map", "0",
        "-c:v", "libx265",
        "-c:a", "copy",
        "-sn",
        "-pix_fmt", get_format_metadata(v["frames"][0]["pix_fmt"].as_str().unwrap()),
    ]);

    let mut width = v["frames"][0]["width"].as_u64().unwrap();
    let mut height = v["frames"][0]["height"].as_u64().unwrap();

    if *is_crop_active {
        let vc = read_crop(input_file);

        width = vc.w;
        height = vc.h;

        ffmpeg.args(&[
            "-vf", &vc.to_ffmpeg_crop_str(),
        ]);
    }

    ffmpeg.args(&[
        "-preset",  &get_preset(
            preset, 
            width,
            height
        ),
    ]);

    ffmpeg.args(&[
        "-crf",  &get_crf(
            crf, 
            width,
            height
        ),
    ]);

    if *is_hdr {
        ffmpeg.args(&[
            "-x265-params",
            &format!(
                "hdr-opt=1:repeat-headers=1:colorprim={colorprim}:transfer={transfer}:colormatrix={colormatrix}:master-display=G({green_x},{green_y})B({blue_x},{blue_y})R({red_x},{red_y})WP({white_point_x},{white_point_y})L({max_luminance},{min_luminance}):max-cll=0,0", 
                colorprim = get_format_metadata(v["frames"][0]["color_primaries"].as_str().unwrap()),
                colormatrix = get_format_metadata(v["frames"][0]["color_space"].as_str().unwrap()),
                transfer = get_format_metadata(v["frames"][0]["color_transfer"].as_str().unwrap()),
                green_x = get_format_metadata(v["frames"][0]["side_data_list"][0]["green_x"].as_str().unwrap()),
                green_y = get_format_metadata(v["frames"][0]["side_data_list"][0]["green_y"].as_str().unwrap()),
                blue_x = get_format_metadata(v["frames"][0]["side_data_list"][0]["blue_x"].as_str().unwrap()),
                blue_y = get_format_metadata(v["frames"][0]["side_data_list"][0]["blue_y"].as_str().unwrap()),
                red_x = get_format_metadata(v["frames"][0]["side_data_list"][0]["red_x"].as_str().unwrap()),
                red_y = get_format_metadata(v["frames"][0]["side_data_list"][0]["red_y"].as_str().unwrap()),
                white_point_x = get_format_metadata(v["frames"][0]["side_data_list"][0]["white_point_x"].as_str().unwrap()),
                white_point_y = get_format_metadata(v["frames"][0]["side_data_list"][0]["white_point_y"].as_str().unwrap()),
                max_luminance = get_format_metadata(v["frames"][0]["side_data_list"][0]["max_luminance"].as_str().unwrap()),
                min_luminance = get_format_metadata(v["frames"][0]["side_data_list"][0]["min_luminance"].as_str().unwrap()),
            ),
        ]);
    }

    ffmpeg.arg(output_file);


    let out = ffmpeg.stdout(Stdio::piped()).spawn();
    
    let reader = BufReader::new(
        out.unwrap().stdout.ok_or_else(|| "Could not capture standard output.").unwrap()
    );

    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| println!("{}", line));
}

fn get_format_metadata(rgb_xy: &str) -> &str {
    let re = regex::Regex::new(r"/").unwrap();
    let fields: Vec<&str> = re.splitn(rgb_xy, 2).collect();

    fields[0]
}

fn get_crf(input_crf: &u8, width: u64, height: u64) -> String {
    if *input_crf > 0 {
        format!("{}", input_crf)
    } else {
        let pixel = width * height;
        match pixel {
            p if p >= 6144000 => {//(>= 3820*1600)
                format!("{}", 13)
            }
            2211841..=6143999 => {
                let range: u64 = 4;
                let diff: u64 = 6143999 - 2211841;

                let step: u64 = diff / range;

                let mut crf = 18;
                let mut temp_pixel: u64 = 2211841;
                
                while pixel > temp_pixel {
                    temp_pixel = temp_pixel + step;
                    crf = crf - 1;

                    println!("crf: {}, pixel: {}, pixel_temp: {}", crf, pixel, temp_pixel);
                }

                format!("{}", crf)
            }
            2073600..=2211840 => {//2K/FHD
                format!("{}", 18)
            }
            1579885..=2073599 => {
                format!("{}", 19)
            }
            _ => {
                format!("{}", 20)
            }
        }
    }
}

fn get_preset(preset: &str, width: u64, height: u64) -> String {
    if preset != "" {
        format!("{}", preset)
    } else {
        let pixel = width * height;
        match pixel {
            p if p >= 8294401 => {//(>= 3820*2160)
                format!("{}", "superfast")
            }
            2211841..=8294400 => {
                format!("{}", "veryfast")
            }
            2073600..=2211840 => {//2K/FHD
                format!("{}", "faster")
            }
            1579885..=2073599 => {
                format!("{}", "fast")
            }
            _ => {
                format!("{}", "medium")
            }
        }
    }
}

fn read_hdr_metadata(input_file: &str) -> serde_json::Value {
    let output = Command::new("ffprobe")
        .args(&[
            "-hide_banner", 
            "-loglevel", "warning",
            "-select_streams", "v",
            "-print_format", "json",
            "-show_frames",
            "-read_intervals", "%+#1",
            "-show_entries", "frame=color_space,color_primaries,color_transfer,side_data_list,pix_fmt,width,height",
            "-i", input_file
        ])
        .output()
        .expect("ffprobe command failed to start");
    
    let out = String::from_utf8(output.stdout).expect("Failed to convert stdout to string.");

    serde_json::from_str(&out).unwrap()
}

fn read_crop(input_file: &str) -> VideoCrop {
    println!("\n\n====> Start Check-Cropfactor");
    let output = Command::new("ffmpeg")
        .args(&[
            "-i", input_file,
            "-ss", "00:05:00",//starttime
            "-t", "00:02:00",//offset
            "-vf", "cropdetect",
            "-f", "null", "-",
        ])
        .output()
        .expect("ffmpeg command failed to start");

    let out = String::from_utf8(output.stderr).expect("Failed to convert stdout to string.");

    let re = regex::Regex::new(r"crop=").unwrap();
    let fields: Vec<&str> = re.splitn(&out, 100).collect();

    let mut crop: VideoCrop = VideoCrop {
        w: 0,
        h: 0,
        x: 0,
        y: 0,
    };

    for i in fields {
        //let test = &i[0..13];
        let crop_temp = VideoCrop::new(
            &i.split(NEW_LINE).next().unwrap()
        );
        
        if let Some(s_corp) = crop_temp {
            if crop.is_smaller_than(&s_corp) {
                crop = s_corp;
            }
        }
    }

    println!("crip: {:?}\n\n\n", crop);

    crop
}

#[derive(Debug)]
struct VideoCrop {
    w: u64,
    h: u64,
    x: u64,
    y: u64
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
    fn is_smaller_than(&self, crop: &VideoCrop) -> bool {
        !(self.w > crop.w && self.h > crop.h && self.x < crop.x && self.y < crop.y)
    }
    fn to_ffmpeg_crop_str(&self) -> String{
        format!("crop={w}:{h}:{x}:{y}",
            w = self.w,
            h = self.h,
            x = self.x,
            y = self.y
        )
    }
}