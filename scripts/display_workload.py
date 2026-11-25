"""Draw a deterministic 30 Hz animation on any X11 server with libX11."""
import ctypes as c
import os
import time


def bind(lib, name, result, *args):
    function = getattr(lib, name)
    function.restype = result
    function.argtypes = args
    return function


def main():
    prefix = os.getenv("PREFIX")
    x11 = c.CDLL(f"{prefix}/lib/libX11.so" if prefix else "libX11.so.6")
    ptr, xid, integer = c.c_void_p, c.c_ulong, c.c_int
    display = bind(x11, "XOpenDisplay", ptr, c.c_char_p)(None)
    if not display:
        raise SystemExit(f"Cannot open {os.getenv('DISPLAY')}")
    root = bind(x11, "XDefaultRootWindow", xid, ptr)(display)
    create = bind(x11, "XCreateSimpleWindow", xid, ptr, xid, integer, integer,
                  c.c_uint, c.c_uint, c.c_uint, xid, xid)
    width, height = 1024, 640
    window = create(display, root, 40, 40, width, height, 0, 0, 0)
    gc = bind(x11, "XCreateGC", ptr, ptr, xid, xid, ptr)(display, window, 0, None)
    foreground = bind(x11, "XSetForeground", integer, ptr, ptr, xid)
    fill = bind(x11, "XFillRectangle", integer, ptr, xid, ptr, integer, integer, c.c_uint, c.c_uint)
    flush = bind(x11, "XFlush", integer, ptr)
    bind(x11, "XStoreName", integer, ptr, xid, c.c_char_p)(display, window, b"Android PC display benchmark")
    bind(x11, "XMapRaised", integer, ptr, xid)(display, window)
    flush(display)
    time.sleep(0.5)
    input_probe = os.getenv("DISPLAY_BENCH_INPUT") == "1"
    if input_probe:
        bind(x11, "XSelectInput", integer, ptr, xid, c.c_long)(display, window, 1 | 4 | 64)
        bind(x11, "XSetInputFocus", integer, ptr, xid, integer, xid)(display, window, 1, 0)
        pending = bind(x11, "XPending", integer, ptr)
        read_event = bind(x11, "XNextEvent", integer, ptr, ptr)
        event = (c.c_long * 24)()
        event_names = {2: "key", 4: "button", 6: "motion"}
        counts = dict.fromkeys(event_names.values(), 0)
        x, y, child = integer(), integer(), xid()
        translate = bind(x11, "XTranslateCoordinates", integer, ptr, xid, xid, integer, integer,
                         c.POINTER(integer), c.POINTER(integer), c.POINTER(xid))
        translate(display, window, root, width // 2, height // 2, c.byref(x), c.byref(y), c.byref(child))
        print(f"Input ready at X11 coordinates {x.value} {y.value}", flush=True)
    frames = int(os.getenv("DISPLAY_BENCH_FRAMES", "360"))
    start = time.monotonic()
    for frame in range(frames):
        time.sleep(max(0, start + frame / 30 - time.monotonic()))
        foreground(display, gc, (frame * 0x050709) & 0xFFFFFF)
        fill(display, window, gc, 0, 0, width, height)
        foreground(display, gc, 0xFFFFFF)
        fill(display, window, gc, frame * 13 % (width - 80), 0, 80, height)
        flush(display)
        if input_probe:
            while pending(display):
                read_event(display, event)
                kind = event_names.get(c.cast(event, c.POINTER(integer))[0])
                if kind:
                    counts[kind] += 1
    bind(x11, "XSync", integer, ptr, integer)(display, 0)
    print(f"Submitted {frames} draws in {time.monotonic() - start:.3f} s", flush=True)
    if input_probe:
        print("X11 events:", counts, flush=True)
    bind(x11, "XCloseDisplay", integer, ptr)(display)


if __name__ == "__main__":
    main()
