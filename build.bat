@echo off
start /wait cargo install --path . --root ./.build
if %errorlevel%==0 (
  mkdir programs
  move .\.build\bin\redstone_compiler.exe .\programs\redc.exe
) else (
  echo If you are on Windows you might want to try this: `rustup default stable-x86_64-pc-windows-gnu`
)
