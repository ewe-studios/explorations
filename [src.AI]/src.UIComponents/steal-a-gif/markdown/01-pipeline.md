# Pipeline Deep Dive

## Decode (`pipeline/decode.ts`)

Decodes video input (MP4, WebM, MOV) into individual frames using the browser's Video API.

## Auto-Crop (`pipeline/autoCrop.ts`)

Analyzes frame boundaries to detect the content area, cropping out unnecessary margins.

## Background Removal (`pipeline/removeBackground.ts`)

Removes or keys out the background, leaving only the animated content.

## Loop Detection (`pipeline/loopDetect.ts`)

Finds optimal loop points by comparing frame similarity across the recording.

## Encode (`pipeline/encodeGif.ts`)

Encodes the processed frames into a GIF using the `gifenc` library.
