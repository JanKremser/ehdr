use std::process::{Command, Stdio};

pub fn extract_rpu(input_file: &str){
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

    let output = Command::new("dovi_tool")
        .args(&[
            "extract-rpu",
            "--rpu-out", &format!("{}.rpu", input_file),
            "-",
        ])
        .stdin(ffmpeg_out)
        .output()
        .expect("ffprobe command failed to start"); 

    let out = String::from_utf8(output.stdout).expect("Failed to convert stdout to string.");

    println!("{:#?}", out);
    //ffmpeg -i GlassBlowing.mp4 -c:v copy -vbsf hevc_mp4toannexb -f hevc - | dovi_tool extract-rpu --rpu-out glass.rpu -
}