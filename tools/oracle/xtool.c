/* xtool — minimal X11 automation helper for the Wine oracle (T0-10).
 *
 * Built against libX11 + libXtst only (xdotool is not installed and there is
 * no sudo on the dev box). Subcommands:
 *   xtool key <keysym-name>      — press & release a key (e.g. "Return", "s")
 *   xtool keydown <name> / keyup <name>
 *   xtool click                  — left button click at current position
 *   xtool move <x> <y>           — warp pointer
 *   xtool clickat <x> <y>        — move + click
 *   xtool shot out.ppm           — grab the root window to a binary PPM
 *   xtool stats                  — print root window size + per-channel means
 *                                  + a coarse brightness histogram (state
 *                                  fingerprinting without vision)
 *   xtool windows                — list window ids + names (via WM_NAME)
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
/* libxtst-dev is not installed on the dev box (no sudo); declare the two
 * functions we need from libXtst.so.6 ourselves. */
extern Bool XTestFakeKeyEvent(Display *, unsigned int, Bool, unsigned long);
extern Bool XTestFakeButtonEvent(Display *, unsigned int, Bool, unsigned long);

static Display *d;
static Window root;
static int x_error_caught = 0;

static int error_handler(Display *dd, XErrorEvent *ev)
{
    (void)dd; (void)ev;
    x_error_caught = 1;
    return 0;
}

static void die(const char *msg) { fprintf(stderr, "xtool: %s\n", msg); exit(1); }

static KeySym keysym(const char *name)
{
    KeySym ks = XStringToKeysym(name);
    if (ks == NoSymbol) die("unknown keysym name");
    return ks;
}

static void send_key(const char *name, int press)
{
    KeyCode code = XKeysymToKeycode(d, keysym(name));
    if (!code) die("no keycode for keysym");
    XTestFakeKeyEvent(d, code, press, CurrentTime);
    XSync(d, False);
}

static XImage *grab(int retries)
{
    XWindowAttributes attr;
    XGetWindowAttributes(d, root, &attr);
    for (int i = 0; i < retries; i++) {
        XSync(d, False);
        x_error_caught = 0;
        XImage *img = XGetImage(d, root, 0, 0, attr.width, attr.height,
                                AllPlanes, ZPixmap);
        XSync(d, False);
        if (img && !x_error_caught)
            return img;
        if (img) XDestroyImage(img);
        usleep(300000); /* mode switches (classic 8bpp fullscreen) can race */
    }
    return NULL;
}

/* Convert an XImage to 24-bit RGB rows, honoring 8-bit palettized roots
 * (classic DirectDraw games run 8bpp; XGetImage then yields palette
 * indices, which we resolve through the installed colormap). */
static unsigned char *to_rgb24(XImage *img, unsigned long *cmap, int cmap_n,
                               int *out_len)
{
    int n = img->width * img->height;
    unsigned char *rgb = malloc(3 * n);
    for (int i = 0; i < n; i++) {
        unsigned long p = XGetPixel(img, i % img->width, i / img->width);
        int r, g, b;
        if (img->depth == 8 && img->bits_per_pixel == 8) {
            unsigned long idx = p & 0xFF;
            if (cmap && idx < (unsigned long)cmap_n) {
                r = cmap[idx * 3] >> 8; g = cmap[idx * 3 + 1] >> 8;
                b = cmap[idx * 3 + 2] >> 8;
            } else { r = g = b = idx; }
        } else {
            r = (p >> 16) & 0xFF; g = (p >> 8) & 0xFF; b = p & 0xFF;
        }
        rgb[i * 3] = r; rgb[i * 3 + 1] = g; rgb[i * 3 + 2] = b;
    }
    *out_len = 3 * n;
    return rgb;
}

static unsigned long *root_cmap(int *n)
{
    XWindowAttributes attr;
    XGetWindowAttributes(d, root, &attr);
    if (attr.depth != 8) { *n = 0; return NULL; }
    Colormap cm;
    int count;
    Colormap *inst = XListInstalledColormaps(d, root, &count);
    cm = (inst && count) ? inst[0] : attr.colormap;
    if (inst) XFree(inst);
    unsigned long *cmap = calloc(256 * 3, sizeof(unsigned long));
    XColor cols[256];
    for (int i = 0; i < 256; i++) cols[i].pixel = i;
    XQueryColors(d, cm, cols, 256);
    for (int i = 0; i < 256; i++) {
        cmap[i * 3] = cols[i].red; cmap[i * 3 + 1] = cols[i].green;
        cmap[i * 3 + 2] = cols[i].blue;
    }
    *n = 256;
    return cmap;
}

int main(int argc, char **argv)
{
    if (argc < 2) die("usage: xtool <key|click|move|clickat|shot|stats|windows> ...");
    d = XOpenDisplay(NULL);
    if (!d) die("cannot open display");
    root = DefaultRootWindow(d);
    XSetErrorHandler(error_handler);
    const char *cmd = argv[1];

    if (!strcmp(cmd, "key") && argc == 3) {
        send_key(argv[2], True); send_key(argv[2], False);
    } else if (!strcmp(cmd, "keydown") && argc == 3) {
        send_key(argv[2], True);
    } else if (!strcmp(cmd, "keyup") && argc == 3) {
        send_key(argv[2], False);
    } else if (!strcmp(cmd, "click")) {
        XTestFakeButtonEvent(d, 1, True, CurrentTime);
        XTestFakeButtonEvent(d, 1, False, CurrentTime);
        XSync(d, False);
    } else if (!strcmp(cmd, "move") && argc == 4) {
        XWarpPointer(d, None, root, 0, 0, 0, 0, atoi(argv[2]), atoi(argv[3]));
        XSync(d, False);
    } else if (!strcmp(cmd, "clickat") && argc == 4) {
        XWarpPointer(d, None, root, 0, 0, 0, 0, atoi(argv[2]), atoi(argv[3]));
        XSync(d, False);
        XTestFakeButtonEvent(d, 1, True, CurrentTime);
        XTestFakeButtonEvent(d, 1, False, CurrentTime);
        XSync(d, False);
    } else if (!strcmp(cmd, "shot") && argc == 3) {
        XWindowAttributes attr;
        XGetWindowAttributes(d, root, &attr);
        XImage *img = grab(6);
        if (!img) die("XGetImage failed (display mode in flux?)");
        FILE *f = fopen(argv[2], "wb");
        if (!f) die("cannot open output");
        int cn; unsigned long *cm = root_cmap(&cn); int rgblen;
        unsigned char *rgb = to_rgb24(img, cm, cn, &rgblen);
        fprintf(f, "P6\n%d %d\n255\n", img->width, img->height);
        fwrite(rgb, 1, rgblen, f);
        free(rgb); free(cm);
        fclose(f);
        XDestroyImage(img);
        printf("shot %dx%d -> %s\n", attr.width, attr.height, argv[2]);
    } else if (!strcmp(cmd, "stats")) {
        XWindowAttributes attr;
        XGetWindowAttributes(d, root, &attr);
        XImage *img = grab(6);
        if (!img) die("XGetImage failed (display mode in flux?)");
        double sr = 0, sg = 0, sb = 0;
        long blue = 0, dark = 0, bright = 0, total = (long)img->width * img->height;
        int cn; unsigned long *cm = root_cmap(&cn); int rgblen;
        unsigned char *rgb = to_rgb24(img, cm, cn, &rgblen);
        int W = img->width;
        for (int y = 0; y < img->height; y += 4)
            for (int x = 0; x < img->width; x += 4) {
                int o = (y * W + x) * 3;
                int r = rgb[o], g = rgb[o + 1], b = rgb[o + 2];
                sr += r; sg += g; sb += b;
                int lum = (r + g + b) / 3;
                if (b > 90 && b > r + 30 && b > g + 20) blue++;
                if (lum < 40) dark++;
                if (lum > 200) bright++;
            }
        long n = ((img->height + 3) / 4) * ((img->width + 3) / 4);
        printf("size %dx%d mean %.0f,%.0f,%.0f blue%% %.0f dark%% %.0f bright%% %.0f\n",
               img->width, img->height, sr / n, sg / n, sb / n,
               100.0 * blue / n, 100.0 * dark / n, 100.0 * bright / n);
        free(rgb); free(cm);
        XDestroyImage(img);
    } else if (!strcmp(cmd, "focusptr")) {
        /* no window manager: make the server deliver keys to whatever window
         * is under the pointer (game DirectDraw windows reject direct focus) */
        XSync(d, False);
        x_error_caught = 0;
        XSetInputFocus(d, PointerRoot, RevertToPointerRoot, CurrentTime);
        XSync(d, False);
        printf("focus follows pointer\n");
    } else if (!strcmp(cmd, "focus") && argc == 3) {
        Window w = (Window)strtoul(argv[2], NULL, 0);
        XSync(d, False);
        x_error_caught = 0;
        XSetInputFocus(d, w, RevertToPointerRoot, CurrentTime);
        XSync(d, False);
        if (x_error_caught) die("XSetInputFocus failed");
        printf("focus set on 0x%lx\n", w);
    } else if (!strcmp(cmd, "windows")) {
        Window r, parent, *children;
        unsigned int n;
        if (!XQueryTree(d, root, &r, &parent, &children, &n))
            die("XQueryTree failed");
        for (unsigned int i = 0; i < n; i++) {
            XTextProperty tp;
            char *name = NULL;
            if (XGetWMName(d, children[i], &tp) && tp.value)
                name = (char *)tp.value;
            XWindowAttributes a;
            XGetWindowAttributes(d, children[i], &a);
            if (a.map_state == IsViewable)
                printf("0x%lx %dx%d+%d+%d %s\n", children[i],
                       a.width, a.height, a.x, a.y, name ? name : "(no name)");
        }
        if (children) XFree(children);
    } else {
        die("bad subcommand or args");
    }
    XCloseDisplay(d);
    return 0;
}
