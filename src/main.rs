use std::string::String;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::fs;

use clap::{Arg, App};

mod video;
use video::Video;

mod dolpy_vision;

fn main() {
    let matches = App::new("SimpleConvert")
        .version("0.2.1")
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
            .help("crf from ffmpeg (default is 'auto')")
        ).arg(Arg::with_name("preset")
            .short("p")
            .long("preset")
            .value_name("STRING")
            .help("preset from ffmpeg (default is 'auto', alternative: [ultrafast, superfast, veryfast, faster, fast, medium])")
        ).arg(Arg::with_name("none-crop")
            .long("ncrop")
            .help("disabled auto-crop function")
        ).arg(Arg::with_name("dolpy-vision")
            .long("dv")
            .help("convert dolpy-vision hdr-video")
        ).get_matches();

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
                            input_file.to_str(), 
                            output_file.to_str(),
                            !matches.is_present("none-crop"),
                            matches.is_present("dolpy-vision"),
                            matches.value_of("crf"),
                            matches.value_of("preset"),
                        );
                    }
                    _ => print!("file is not supported")
                }
            }
        }
    } else {
        convert(
            matches.value_of("input"), 
            matches.value_of("output"),
            !matches.is_present("none-crop"),
            matches.is_present("dolpy-vision"),
            matches.value_of("crf"),
            matches.value_of("preset"),
        );
    }
}

fn convert(input_file: Option<&str>, output_file: Option<&str>, is_crop: bool, is_dolpy_vision: bool, crf: Option<&str>, preset: Option<&str>) -> Option<bool> {
    let input_file = Path::new(
        input_file?
    );

    let mut input_video = Video::new(&input_file)?;

    if is_crop {
        input_video.crop_video();
    }

    let target_crf: String;
    if let Some(out_crf) = crf {
        target_crf = out_crf.to_string();
    } else {
        target_crf = input_video.get_auto_crf();
    }

    let target_preset: String;
    if let Some(out_preset) = preset {
        target_preset = out_preset.to_string();
    } else {
        target_preset = input_video.get_auto_preset();
    }

    if is_dolpy_vision {
        convert_dolpy_vision(&input_video, output_file?, &target_crf, &target_preset)
    }else{
        convert_sdr_hdr10(&input_video, output_file?, &target_crf, &target_preset);
    }


    Some(true)
}

fn convert_dolpy_vision(input_video: &Video, output_file: &str, crf: &str, preset: &str) {
    let input_file = input_video.get_path_str().unwrap();
    dolpy_vision::extract_rpu(input_file);

    let ffmpeg_out = Command::new("ffmpeg")
        .args(&[
            "-i", input_file,
            "-f", "yuv4mpegpipe",
            "-strict", "-1",
            "-pix_fmt", input_video.get_pix_fmt().unwrap(),
            "-",
        ])
        .stdout(Stdio::piped())
        .spawn().unwrap().stdout.expect("ffmpeg command failed to start");

    let output = Command::new("x265")
        .args(&[
            "-",
            "--input-depth", "10",//10bit
            "--output-depth", "10",//10bit
            "--y4m",
            "--preset", preset,
            "--crf", crf,
            "--master-display", &input_video.get_master_display(),
            "--max-cll", "0,0",
            "--colormatrix", input_video.get_color_space().unwrap(),
            "--colorprim", input_video.get_color_primaries().unwrap(),
            "--transfer", input_video.get_color_transfer().unwrap(),
            "--dolby-vision-rpu", &format!("{}.rpu", input_file),
            "--dolby-vision-profile", "8.1",
            "--vbv-bufsize", "20000",
            "--vbv-maxrate", "20000",
            &format!("{}.hevc", output_file),
        ])
        .stdin(ffmpeg_out)
        .output()
        .expect("x265 command failed to start"); 

    let out = String::from_utf8(output.stdout).expect("Failed to convert stdout to string.");

    println!("{:#?}", out);
}

fn convert_sdr_hdr10(input_video: &Video, output_file: &str, crf: &str, preset: &str) {
    println!("---Metadata: \n{:#?}", input_video);

    let mut ffmpeg: Command = Command::new("ffmpeg");
    ffmpeg.args(&[
        "-i", input_video.get_path_str().unwrap(),
        "-map", "0:v",
        "-map", "0:a?",
        "-map", "0:s?",
        "-c:a", "copy",
        "-c:s", "copy",
        "-c:v", "libx265",
        "-pix_fmt", input_video.get_pix_fmt().unwrap(),
    ]);

    if input_video.is_croped_video() {
        ffmpeg.args(&[
            "-vf", &input_video.get_ffmpeg_crop_str(),
        ]);
    }

    ffmpeg.args(&[
        "-preset",  preset,
        "-crf", crf,
    ]);

    if input_video.is_hdr_video() {
        println!("=== HDR video detected ===");
        ffmpeg.args(&[
            "-x265-params",
            &format!(
                "hdr-opt=1:repeat-headers=1:colorprim={colorprim}:transfer={transfer}:colormatrix={colormatrix}:master-display={master_display}:max-cll=0,0", 
                colorprim = input_video.get_color_primaries().unwrap(),
                colormatrix = input_video.get_color_space().unwrap(),
                transfer = input_video.get_color_transfer().unwrap(),
                master_display = &input_video.get_master_display(),
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
