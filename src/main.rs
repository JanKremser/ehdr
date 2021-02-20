use std::string::String;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::fs;

use clap::{Arg, App};

mod video;
use video::Video;

fn main() {
    let matches = App::new("SimpleConvert")
        .version("0.2.0")
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

                        let mut input_video = Video::new(&input_file).unwrap();

                        convert(
                            &mut input_video, 
                            output_file.to_str().unwrap(),
                            &matches.is_present("crop"),
                            &crf,
                            preset,
                        );
                    }
                    _ => print!("file is not suportet")
                }
            }
        }
    } else {
        let input_file = Path::new(matches.value_of("input").unwrap());
        let mut input_video = Video::new(&input_file).unwrap();

        convert(
            &mut input_video, 
            matches.value_of("output").unwrap(),
            &matches.is_present("crop"),
            &crf,
            preset,
        );
    }
}

fn convert(input_video: &mut Video, output_file: &str, is_crop_active: &bool, crf: &u8, preset: &str) {
    println!("---Metadata: \n{:#?}", input_video);

    let mut ffmpeg: Command = Command::new("ffmpeg");
    ffmpeg.args(&[
        "-i", input_video.get_path_str().unwrap(),
        "-map", "0",
        "-c:v", "libx265",
        "-c:a", "copy",
        "-sn",
        "-pix_fmt", input_video.get_pix_fmt().unwrap(),
    ]);

    if *is_crop_active {
        input_video.crop_video();

        ffmpeg.args(&[
            "-vf", &input_video.get_ffmpeg_crop_str(),
        ]);
    }

    ffmpeg.args(&[
        "-preset",  &get_preset(
            preset, 
            input_video.get_width(),
            input_video.get_height(),
        ),
    ]);

    ffmpeg.args(&[
        "-crf",  &get_crf(
            crf, 
            input_video.get_width(),
            input_video.get_height(),
        ),
    ]);

    if input_video.is_hdr_video() {
        println!("=== HDR video detected ===");
        ffmpeg.args(&[
            "-x265-params",
            &format!(
                "hdr-opt=1:repeat-headers=1:colorprim={colorprim}:transfer={transfer}:colormatrix={colormatrix}:master-display=G({green_x},{green_y})B({blue_x},{blue_y})R({red_x},{red_y})WP({white_point_x},{white_point_y})L({max_luminance},{min_luminance}):max-cll=0,0", 
                colorprim = input_video.get_color_primaries().unwrap(),
                colormatrix = input_video.get_color_space().unwrap(),
                transfer = input_video.get_color_transfer().unwrap(),
                green_x = input_video.get_side_data_list_param("green_x").unwrap(),
                green_y = input_video.get_side_data_list_param("green_y").unwrap(),
                blue_x = input_video.get_side_data_list_param("blue_x").unwrap(),
                blue_y = input_video.get_side_data_list_param("blue_y").unwrap(),
                red_x = input_video.get_side_data_list_param("red_x").unwrap(),
                red_y = input_video.get_side_data_list_param("red_y").unwrap(),
                white_point_x = input_video.get_side_data_list_param("white_point_x").unwrap(),
                white_point_y = input_video.get_side_data_list_param("white_point_y").unwrap(),
                max_luminance = input_video.get_side_data_list_param("max_luminance").unwrap(),
                min_luminance = input_video.get_side_data_list_param("min_luminance").unwrap(),
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
