use std::process::{Command, Stdio};
use std::path::Path;

pub fn extract_rpu(input_file: &str) {
    let output_rpufile = format!("{}.rpu", input_file);

    let rpufile = Path::new(&output_rpufile);

    if rpufile.exists() {
        println!("skip extract_rpu: rpu file exists");
        return;
    }

    let ffmpeg_out = Command::new("ffmpeg")
        .args(&[
            "-i", input_file,
            "-c:v", "copy",
            "-vbsf", "hevc_mp4toannexb",
            "-f", "hevc",
            "-",
        ])
        .stdout(Stdio::piped())
        .spawn().unwrap().stdout.expect("ffprobe command failed to start");

    Command::new("dovi_tool")
        .args(&[
            "-m", "2",//to 8.1
            "extract-rpu",
            "--rpu-out", &output_rpufile,
            "-",
        ])
        .stdin(ffmpeg_out)
        .output()
        .expect("ffprobe command failed to start"); 

    //ffmpeg -i GlassBlowing.mp4 -c:v copy -vbsf hevc_mp4toannexb -f hevc - | dovi_tool extract-rpu --rpu-out glass.rpu -
}