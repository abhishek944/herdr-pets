#!/usr/bin/env python3
"""Validate bundled pet APNG structure and frame payloads without dependencies."""

from __future__ import annotations

import binascii
import json
import struct
import sys
import tempfile
import zlib
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PETS = ROOT / "src" / "pets"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
KNOWN_CRITICAL_CHUNKS = {b"IHDR", b"PLTE", b"IDAT", b"IEND"}


def fail(path: Path, message: str) -> None:
    try:
        display = path.relative_to(ROOT)
    except ValueError:
        display = path
    raise ValueError(f"{display}: {message}")


def chunks(path: Path):
    payload = path.read_bytes()
    if not payload.startswith(PNG_SIGNATURE):
        fail(path, "invalid PNG signature")
    offset = len(PNG_SIGNATURE)
    seen_iend = False
    while offset < len(payload):
        if seen_iend:
            fail(path, "data appears after IEND")
        if offset + 12 > len(payload):
            fail(path, "truncated PNG chunk")
        length = struct.unpack(">I", payload[offset : offset + 4])[0]
        kind = payload[offset + 4 : offset + 8]
        end = offset + 12 + length
        if end > len(payload):
            fail(path, "truncated PNG payload")
        data = payload[offset + 8 : offset + 8 + length]
        expected = struct.unpack(">I", payload[offset + 8 + length : end])[0]
        actual = binascii.crc32(kind + data) & 0xFFFFFFFF
        if actual != expected:
            fail(path, f"invalid {kind.decode('ascii', 'replace')} checksum")
        if any(byte not in range(65, 91) and byte not in range(97, 123) for byte in kind):
            fail(path, "chunk type must contain only ASCII letters")
        if kind[2] & 0x20:
            fail(path, f"chunk {kind.decode()} uses the reserved lowercase bit")
        if not kind[0] & 0x20 and kind not in KNOWN_CRITICAL_CHUNKS:
            fail(path, f"unknown critical chunk {kind.decode()}")
        if kind == b"IEND":
            if data:
                fail(path, "IEND must be empty")
            seen_iend = True
        yield kind, data
        offset = end
    if not seen_iend:
        fail(path, "missing IEND")


def paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    distances = (abs(estimate - left), abs(estimate - above), abs(estimate - upper_left))
    return (left, above, upper_left)[distances.index(min(distances))]


def frame_payload(path: Path, width: int, height: int, compressed: list[bytes]) -> tuple[int, bytes]:
    try:
        raw = zlib.decompress(b"".join(compressed))
    except zlib.error as error:
        fail(path, f"invalid compressed frame: {error}")
    stride = width * 4
    if len(raw) != (stride + 1) * height:
        fail(path, "unexpected RGBA frame payload size")
    previous = bytearray(stride)
    visible = 0
    decoded = bytearray()
    for row_index in range(height):
        start = row_index * (stride + 1)
        filter_type = raw[start]
        source = raw[start + 1 : start + 1 + stride]
        row = bytearray(stride)
        for index, value in enumerate(source):
            left = row[index - 4] if index >= 4 else 0
            above = previous[index]
            upper_left = previous[index - 4] if index >= 4 else 0
            if filter_type == 0:
                prediction = 0
            elif filter_type == 1:
                prediction = left
            elif filter_type == 2:
                prediction = above
            elif filter_type == 3:
                prediction = (left + above) // 2
            elif filter_type == 4:
                prediction = paeth(left, above, upper_left)
            else:
                fail(path, f"unsupported PNG filter {filter_type}")
            row[index] = (value + prediction) & 0xFF
        visible += sum(alpha > 0 for alpha in row[3::4])
        decoded.extend(row)
        previous = row
    return visible, bytes(decoded)


def composite_over(destination: bytes, source: bytes) -> bytes:
    source_alpha = source[3]
    if source_alpha == 0:
        return destination
    if source_alpha == 255:
        return source
    destination_alpha = destination[3]
    output_alpha = source_alpha + destination_alpha * (255 - source_alpha) // 255
    if output_alpha == 0:
        return b"\0\0\0\0"
    channels = [
        (source[index] * source_alpha * 255 + destination[index] * destination_alpha * (255 - source_alpha))
        // (output_alpha * 255)
        for index in range(3)
    ]
    return bytes((*channels, output_alpha))


def normalized_canvas(canvas: bytearray) -> bytes:
    output = bytearray(canvas)
    for index in range(0, len(output), 4):
        if output[index + 3] == 0:
            output[index : index + 3] = b"\0\0\0"
    return bytes(output)


def render_frame(canvas: bytearray, canvas_width: int, control: tuple, payload: bytes) -> None:
    _, width, height, x, y, _, _, _, blend = control
    for row in range(height):
        for column in range(width):
            source_index = (row * width + column) * 4
            target_index = ((y + row) * canvas_width + x + column) * 4
            source = payload[source_index : source_index + 4]
            if blend == 0:
                canvas[target_index : target_index + 4] = source
            else:
                destination = bytes(canvas[target_index : target_index + 4])
                canvas[target_index : target_index + 4] = composite_over(destination, source)


def clear_frame_area(canvas: bytearray, canvas_width: int, control: tuple) -> None:
    _, width, height, x, y, _, _, _, _ = control
    clear_row = b"\0" * (width * 4)
    for row in range(height):
        start = ((y + row) * canvas_width + x) * 4
        canvas[start : start + len(clear_row)] = clear_row


def validate_apng(path: Path, expected_duration_ms: int | None = None) -> None:
    canvas = None
    animation = None
    frames: list[dict] = []
    seen_data = False
    seen_fdat = False
    seen_idat = False
    idat_closed = False
    next_sequence = 0
    for chunk_index, (kind, data) in enumerate(chunks(path)):
        if chunk_index == 0 and kind != b"IHDR":
            fail(path, "IHDR must be the first chunk")
        if seen_idat and kind != b"IDAT":
            idat_closed = True
        if kind == b"IHDR":
            if canvas is not None:
                fail(path, "duplicate IHDR")
            if len(data) != 13:
                fail(path, "IHDR payload length is invalid")
            width, height, depth, color, compression, filter_method, interlace = struct.unpack(">IIBBBBB", data)
            canvas = (width, height)
            if depth != 8 or color != 6:
                fail(path, "assets must use 8-bit RGBA pixels")
            if compression != 0 or filter_method != 0 or interlace != 0:
                fail(path, "assets must use standard compression, filtering, and no interlace")
            if width < 1 or height < 1 or width > 2048 or height > 2048:
                fail(path, "canvas dimensions are outside 1..2048")
        elif kind == b"acTL":
            if animation is not None or seen_data or len(data) != 8:
                fail(path, "acTL must appear once before image data with a valid payload")
            animation = struct.unpack(">II", data)
        elif kind == b"fcTL":
            if animation is None or len(data) != 26 or (frames and not frames[-1]["data"]):
                fail(path, "frame control ordering or payload is invalid")
            control = struct.unpack(">IIIIIHHBB", data)
            sequence, width, height, x, y, _, _, disposal, blend = control
            if not frames and canvas != (width, height) or (not frames and (x != 0 or y != 0)):
                fail(path, "the default first APNG frame must fill the canvas")
            if sequence != next_sequence:
                fail(path, f"expected APNG sequence {next_sequence}, found {sequence}")
            if disposal not in (0, 1, 2) or blend not in (0, 1):
                fail(path, "invalid APNG disposal or blend operation")
            next_sequence += 1
            frames.append({"control": control, "data": []})
        elif kind == b"IDAT":
            if animation is None or len(frames) != 1 or seen_fdat or idat_closed:
                fail(path, "IDAT is allowed only in one consecutive first-frame run")
            seen_data = True
            seen_idat = True
            frames[0]["data"].append(data)
        elif kind == b"fdAT":
            if not frames or len(frames) < 2 or len(data) < 5:
                fail(path, "frame data appears before frame control")
            sequence = struct.unpack(">I", data[:4])[0]
            if sequence != next_sequence:
                fail(path, f"expected APNG sequence {next_sequence}, found {sequence}")
            next_sequence += 1
            seen_data = True
            seen_fdat = True
            frames[-1]["data"].append(data[4:])
        elif kind == b"IEND" and frames and not frames[-1]["data"]:
            fail(path, "final frame has no image data")
    if canvas is None or animation is None:
        fail(path, "asset is not an APNG")
    declared_frames, plays = animation
    if declared_frames < 2 or declared_frames != len(frames):
        fail(path, "APNG frame count is invalid")
    if plays != 0:
        fail(path, "APNG must loop indefinitely")
    canvas_width, canvas_height = canvas
    canvas_payload = bytearray(canvas_width * canvas_height * 4)
    frame_signatures = set()
    previous_display = None
    total_duration = Fraction(0)
    for index, frame in enumerate(frames, 1):
        control = frame["control"]
        _, width, height, x, y, numerator, denominator, disposal, _ = control
        if width < 1 or height < 1 or x + width > canvas_width or y + height > canvas_height:
            fail(path, f"frame {index} exceeds the canvas")
        if numerator == 0:
            fail(path, f"frame {index} has zero duration")
        visible, payload = frame_payload(path, width, height, frame["data"])
        if visible < max(16, width * height // 1000):
            fail(path, f"frame {index} is effectively blank ({visible} visible pixels)")
        before_frame = canvas_payload.copy()
        render_frame(canvas_payload, canvas_width, control, payload)
        displayed = normalized_canvas(canvas_payload)
        alphas = displayed[3::4]
        transparent = sum(alpha == 0 for alpha in alphas)
        if transparent < max(16, canvas_width * canvas_height // 100):
            fail(path, f"frame {index} lacks a meaningful fully transparent area")
        if previous_display is not None:
            changed = sum(
                displayed[offset : offset + 4] != previous_display[offset : offset + 4]
                for offset in range(0, len(displayed), 4)
            )
            if changed < max(16, canvas_width * canvas_height // 5000):
                fail(path, f"frame {index} has too little visible change ({changed} pixels)")
        frame_signatures.add(binascii.crc32(displayed))
        previous_display = displayed
        total_duration += Fraction(numerator * 1000, denominator or 100)
        if disposal == 1:
            clear_frame_area(canvas_payload, canvas_width, control)
        elif disposal == 2:
            canvas_payload = before_frame
    if len(frame_signatures) < 2:
        fail(path, "APNG displayed frames do not change")
    if expected_duration_ms is not None and total_duration != expected_duration_ms:
        fail(path, f"APNG duration {float(total_duration):g}ms does not match clip duration {expected_duration_ms}ms")


def validate_still_png(path: Path) -> None:
    canvas = None
    compressed = []
    seen_idat = False
    idat_closed = False
    for chunk_index, (kind, data) in enumerate(chunks(path)):
        if chunk_index == 0 and kind != b"IHDR":
            fail(path, "IHDR must be the first chunk")
        if seen_idat and kind != b"IDAT":
            idat_closed = True
        if kind == b"IHDR":
            if canvas is not None or len(data) != 13:
                fail(path, "static PNG has an invalid IHDR")
            width, height, depth, color, compression, filter_method, interlace = struct.unpack(">IIBBBBB", data)
            if (
                depth != 8 or color != 6 or compression != 0 or filter_method != 0 or interlace != 0
                or width < 1 or height < 1 or width > 2048 or height > 2048
            ):
                fail(path, "static PNG must use safe non-interlaced 8-bit RGBA dimensions")
            canvas = (width, height)
        elif kind == b"IDAT":
            if idat_closed:
                fail(path, "static PNG IDAT chunks must be consecutive")
            seen_idat = True
            compressed.append(data)
        elif kind in (b"acTL", b"fcTL", b"fdAT"):
            fail(path, "hold asset must be a static PNG")
    if canvas is None or not compressed:
        fail(path, "static PNG has no image payload")
    width, height = canvas
    visible, payload = frame_payload(path, width, height, compressed)
    if visible < max(16, width * height // 1000):
        fail(path, "static PNG is effectively blank")
    transparent = sum(alpha == 0 for alpha in payload[3::4])
    if transparent < max(16, width * height // 100):
        fail(path, "static PNG lacks a meaningful fully transparent area")


def mutate_chunk(payload: bytes, target: bytes, mutate) -> bytes:
    output = bytearray(payload)
    offset = len(PNG_SIGNATURE)
    while offset < len(output):
        length = struct.unpack(">I", output[offset : offset + 4])[0]
        kind = bytes(output[offset + 4 : offset + 8])
        if kind == target:
            data_start = offset + 8
            data = mutate(bytes(output[data_start : data_start + length]))
            output[data_start : data_start + length] = data
            crc = binascii.crc32(kind + data) & 0xFFFFFFFF
            output[data_start + length : data_start + length + 4] = struct.pack(">I", crc)
            return bytes(output)
        offset += 12 + length
    raise ValueError(f"fixture has no {target.decode()} chunk")


def expect_rejected(path: Path) -> None:
    try:
        validate_apng(path)
    except ValueError:
        return
    raise ValueError(f"negative APNG fixture was accepted: {path.name}")


def expect_still_rejected(path: Path) -> None:
    try:
        validate_still_png(path)
    except ValueError:
        return
    raise ValueError(f"negative static PNG fixture was accepted: {path.name}")


def fixture_chunk(kind: bytes, data: bytes) -> bytes:
    checksum = binascii.crc32(kind + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", checksum)


def fixture_frame(width: int, height: int, pixel) -> bytes:
    rows = []
    for y in range(height):
        rows.append(b"\0" + b"".join(bytes(pixel(x, y)) for x in range(width)))
    return zlib.compress(b"".join(rows))


def insert_chunk_after(payload: bytes, target: bytes, kind: bytes, data: bytes = b"") -> bytes:
    offset = len(PNG_SIGNATURE)
    while offset < len(payload):
        length = struct.unpack(">I", payload[offset : offset + 4])[0]
        end = offset + 12 + length
        if payload[offset + 4 : offset + 8] == target:
            return payload[:end] + fixture_chunk(kind, data) + payload[end:]
        offset = end
    raise ValueError(f"fixture has no {target.decode()} chunk")


def write_fixture_still(path: Path, split_idat: bool) -> None:
    width = height = 16
    pixel = lambda x, y: (255, 0, 0, 255) if y == 0 else (0, 0, 0, 0)
    compressed = fixture_frame(width, height, pixel)
    midpoint = len(compressed) // 2
    payload = PNG_SIGNATURE + fixture_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    payload += fixture_chunk(b"IDAT", compressed[:midpoint] if split_idat else compressed)
    if split_idat:
        payload += fixture_chunk(b"tEXt", b"gap") + fixture_chunk(b"IDAT", compressed[midpoint:])
    payload += fixture_chunk(b"IEND", b"")
    path.write_bytes(payload)


def write_fixture_apng(path: Path, first_pixel, second_pixel, split_idat: bool = False) -> None:
    width = height = 16
    payload = PNG_SIGNATURE
    payload += fixture_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    payload += fixture_chunk(b"acTL", struct.pack(">II", 2, 0))
    payload += fixture_chunk(b"fcTL", struct.pack(">IIIIIHHBB", 0, width, height, 0, 0, 1, 10, 0, 0))
    first_payload = fixture_frame(width, height, first_pixel)
    if split_idat:
        midpoint = len(first_payload) // 2
        payload += fixture_chunk(b"IDAT", first_payload[:midpoint])
        payload += fixture_chunk(b"tEXt", b"gap")
        payload += fixture_chunk(b"IDAT", first_payload[midpoint:])
    else:
        payload += fixture_chunk(b"IDAT", first_payload)
    payload += fixture_chunk(b"fcTL", struct.pack(">IIIIIHHBB", 1, width, height, 0, 0, 1, 10, 0, 0))
    payload += fixture_chunk(b"fdAT", struct.pack(">I", 2) + fixture_frame(width, height, second_pixel))
    payload += fixture_chunk(b"IEND", b"")
    path.write_bytes(payload)


def mutate_chunk_kind(payload: bytes, target: bytes, replacement: bytes) -> bytes:
    output = bytearray(payload)
    offset = len(PNG_SIGNATURE)
    while offset < len(output):
        length = struct.unpack(">I", output[offset : offset + 4])[0]
        kind_start = offset + 4
        if bytes(output[kind_start : kind_start + 4]) == target:
            data = bytes(output[offset + 8 : offset + 8 + length])
            output[kind_start : kind_start + 4] = replacement
            checksum = binascii.crc32(replacement + data) & 0xFFFFFFFF
            output[offset + 8 + length : offset + 12 + length] = struct.pack(">I", checksum)
            return bytes(output)
        offset += 12 + length
    raise ValueError(f"fixture has no {target.decode()} chunk")


def self_test(source: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="herdr-pets-apng-") as directory:
        temp = Path(directory)
        payload = source.read_bytes()
        unknown_critical = temp / "unknown-critical.png"
        unknown_critical.write_bytes(insert_chunk_after(payload, b"IHDR", b"ABCD"))
        expect_rejected(unknown_critical)

        split_still = temp / "split-static-idat.png"
        write_fixture_still(split_still, split_idat=True)
        expect_still_rejected(split_still)

        missing_iend = temp / "missing-iend.png"
        missing_iend.write_bytes(payload[:-12])
        expect_rejected(missing_iend)

        bad_sequence = temp / "bad-sequence.png"
        bad_sequence.write_bytes(mutate_chunk(payload, b"fcTL", lambda data: struct.pack(">I", 99) + data[4:]))
        expect_rejected(bad_sequence)

        partial_first = temp / "partial-first-frame.png"
        partial_first.write_bytes(mutate_chunk(payload, b"fcTL", lambda data: data[:4] + struct.pack(">I", 1) + data[8:]))
        expect_rejected(partial_first)

        bad_disposal = temp / "bad-disposal.png"
        bad_disposal.write_bytes(mutate_chunk(payload, b"fcTL", lambda data: data[:-2] + bytes((9, data[-1]))))
        expect_rejected(bad_disposal)

        bad_filter = temp / "bad-filter.png"
        bad_filter.write_bytes(mutate_chunk(payload, b"IHDR", lambda data: data[:11] + b"\1" + data[12:]))
        expect_rejected(bad_filter)

        bad_interlace = temp / "bad-interlace.png"
        bad_interlace.write_bytes(mutate_chunk(payload, b"IHDR", lambda data: data[:12] + b"\1"))
        expect_rejected(bad_interlace)

        later_idat = temp / "later-frame-idat.png"
        later_idat.write_bytes(mutate_chunk_kind(payload, b"fdAT", b"IDAT"))
        expect_rejected(later_idat)

        try:
            validate_apng(source, 1)
        except ValueError:
            pass
        else:
            raise ValueError("APNG duration mismatch was accepted")

        opaque = temp / "opaque.png"
        write_fixture_apng(opaque, lambda x, y: (255, 0, 0, 255), lambda x, y: (0, 0, 255, 255))
        expect_rejected(opaque)

        near_opaque = temp / "near-opaque.png"
        almost_red = lambda x, y: (255, 0, 0, 254 if x == 0 and y == 0 else 255)
        almost_blue = lambda x, y: (0, 0, 255, 254 if x == 0 and y == 0 else 255)
        write_fixture_apng(near_opaque, almost_red, almost_blue)
        expect_rejected(near_opaque)

        nonconsecutive = temp / "nonconsecutive-idat.png"
        write_fixture_apng(nonconsecutive, almost_red, almost_blue, split_idat=True)
        expect_rejected(nonconsecutive)

        hidden_rgb = temp / "hidden-rgb-only.png"
        first = lambda x, y: (255, 0, 0, 255) if y == 0 else (0, 0, 0, 0)
        second = lambda x, y: (255, 0, 0, 255) if y == 0 else (0, 255, 0, 0)
        write_fixture_apng(hidden_rgb, first, second)
        expect_rejected(hidden_rgb)

        one_pixel = temp / "one-pixel-change.png"
        changed = lambda x, y: (0, 0, 255, 255) if x == 0 and y == 0 else first(x, y)
        write_fixture_apng(one_pixel, first, changed)
        expect_rejected(one_pixel)


def main() -> None:
    manifests = sorted(PETS.glob("*/flow.json"))
    if not manifests:
        raise ValueError("src/pets must contain at least one flow.json")
    animations: dict[Path, int] = {}
    stills: set[Path] = set()
    for manifest_path in manifests:
        manifest = json.loads(manifest_path.read_text())
        for clip in manifest.get("clips", {}).values():
            asset = clip.get("asset")
            if asset:
                path = (manifest_path.parent / asset).resolve()
                if manifest_path.parent.resolve() not in path.parents:
                    fail(manifest_path, f"unsafe asset path {asset}")
                duration = clip.get("durationMs")
                if path in animations and animations[path] != duration:
                    fail(manifest_path, f"shared asset {asset} has conflicting clip durations")
                if path not in animations:
                    validate_apng(path, duration)
                    animations[path] = duration
            hold_asset = clip.get("holdAsset")
            if hold_asset:
                path = (manifest_path.parent / hold_asset).resolve()
                if manifest_path.parent.resolve() not in path.parents:
                    fail(manifest_path, f"unsafe hold asset path {hold_asset}")
                if path not in stills:
                    validate_still_png(path)
                    stills.add(path)
    self_test(next(iter(animations)))
    print(f"APNG asset checks: pass ({len(animations)} animations, {len(stills)} hold stills, negative fixtures rejected)")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, struct.error, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
