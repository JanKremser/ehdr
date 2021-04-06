# ehdr [![Rustc Version 1.48+]][rustc]

[Rustc Version 1.48+]: https://img.shields.io/badge/rustc-1.48+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2020/11/19/Rust-1.48.html


**easy hdr(HDR10/Dolpy Vision) / sdr video converter**

---

All videos are converted to h265(hevc). The combination settings are set dynamically based on the resolution. These can be influenced with the parameters `--crf` and `--preset`. Read the [ffmpeg](https://trac.ffmpeg.org/wiki/Encode/H.265) documentation for this.

## Dependencies:
* [ffmpeg / ffprobe](https://ffmpeg.org/download.html)

| only for dolby vision:
* [x265](https://github.com/videolan/x265) (10bit) \
    [for windows](http://msystem.waw.pl/x265/)
* [dovi_tool](https://github.com/quietvoid/dovi_tool)

## Functions:

Videos with black bars are automatically cropped. If you do not want this use `--ncrop`.

Easy conversion of hdr10 or sdr content:
```bash
ehdr -i input.mkv -o out.mkv
```

Dolpy vision is not yet automatically detected. please use the parameter `--dv` for dolpy vision videos. For dolpy vision no crop is currently supported. please use `--ncrop`!
```bash
ehdr -i input.mkv -o out.mkv --dv --ncrop
```

Convert multiple files one by one:
```bash
ehdr -i ./input_folder -o ./output_folder 
```
