# otterpack

otterpack is a self-extracting Windows executable, used for [Craig](https://craig.chat)'s Windows Executable download format.

To create a self-extracting executable, you can build this executable with `cargo build --release` and append a ZIP file to it. otterpack will search and unzip the bundled ZIP file when ran. The ZIP file must have FLAC files in the root folder along with an `ffmpeg.exe` in the ZIP to use. You can also run [UPX](https://upx.github.io) on the compiled binary before merging.

##### Windows
```bat
copy /b otterpack.exe + recording.zip otterpack-packed.exe
```

##### Linux
```sh
cat otterpack.exe recording.zip > otterpack-packed.exe
```

### Why?
- I wouldn't know how to properly update the previous self-extractor. The previous version used fluid and unzip, and although it is smaller, I think this might be more managable and maintainable.
- I may want to add on to the extractor and add more features.
- I wanted to try out Rust some more.

---
This is project is licensed under [MIT](/LICENSE).