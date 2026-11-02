/* winject — Win32-level input injection for the Wine oracle (T0-10).
 *
 * Runs inside the wine prefix and injects via SendInput/SetCursorPos, which
 * the game treats as real input (XTest events never registered on the
 * ToTPP launcher's DirectDraw UI). Coordinates are wine-desktop-space.
 *
 *   winject key <vk>          press & release a virtual key
 *   winject keydown/up <vk>
 *   winject click             left click at current position
 *   winject dblclick
 *   winject move <x> <y>
 *   winject clickat <x> <y>   move + click
 *   winject dblclickat <x> <y>
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

static void die(const char *msg)
{
    fprintf(stderr, "winject: %s\n", msg);
    exit(1);
}

static void send_key(WORD vk, BOOL up)
{
    INPUT in = {0};
    in.type = INPUT_KEYBOARD;
    in.ki.wVk = vk;
    in.ki.dwFlags = up ? KEYEVENTF_KEYUP : 0;
    if (!SendInput(1, &in, sizeof(INPUT)))
        die("SendInput(key) failed");
}

static void click(void)
{
    INPUT down = {0}, up = {0};
    down.type = up.type = INPUT_MOUSE;
    down.mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
    up.mi.dwFlags = MOUSEEVENTF_LEFTUP;
    if (!SendInput(1, &down, sizeof(INPUT)) || !SendInput(1, &up, sizeof(INPUT)))
        die("SendInput(mouse) failed");
}

static void move_to(int x, int y)
{
    if (!SetCursorPos(x, y))
        die("SetCursorPos failed");
}

static void nap(int ms) { Sleep(ms); }

static BOOL CALLBACK enum_cb(HWND hwnd, LPARAM lp)
{
    char title[256];
    DWORD pid = 0;
    GetWindowTextA(hwnd, title, sizeof(title));
    GetWindowThreadProcessId(hwnd, &pid);
    if (title[0] || IsWindowVisible(hwnd)) {
        RECT r;
        GetWindowRect(hwnd, &r);
        printf("hwnd=%08lx pid=%lu vis=%d rect=%ld,%ld %ldx%ld %s\n",
               (unsigned long)(DWORD_PTR)hwnd, (unsigned long)pid,
               IsWindowVisible(hwnd), r.left, r.top,
               r.right - r.left, r.bottom - r.top, title);
    }
    (void)lp;
    return TRUE;
}

int main(int argc, char **argv)
{
    if (argc < 2)
        die("usage: winject key|click|dblclick|move|clickat|dblclickat|wins|post ...");
    const char *cmd = argv[1];

    if (!strcmp(cmd, "wins") && argc == 2) {
        EnumWindows(enum_cb, 0);
    } else if (!strcmp(cmd, "post") && argc == 5) {
        /* post <hwnd-hex> <x> <y> — WM_LBUTTONDOWN/UP in client coords */
        HWND h = (HWND)(DWORD_PTR)strtoul(argv[2], NULL, 16);
        int x = atoi(argv[3]), y = atoi(argv[4]);
        if (!h || !IsWindow(h))
            die("bad hwnd");
        PostMessageW(h, WM_MOUSEMOVE, 0, MAKELPARAM(x, y));
        Sleep(80);
        PostMessageW(h, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(x, y));
        Sleep(60);
        PostMessageW(h, WM_LBUTTONUP, 0, MAKELPARAM(x, y));
        printf("posted to %08lx at %d,%d\n",
               (unsigned long)(DWORD_PTR)h, x, y);
    } else if (!strcmp(cmd, "key") && argc == 3) {
        send_key((WORD)atoi(argv[2]), FALSE);
        nap(30);
        send_key((WORD)atoi(argv[2]), TRUE);
    } else if (!strcmp(cmd, "keydown") && argc == 3) {
        send_key((WORD)atoi(argv[2]), FALSE);
    } else if (!strcmp(cmd, "keyup") && argc == 3) {
        send_key((WORD)atoi(argv[2]), TRUE);
    } else if (!strcmp(cmd, "click") && argc == 2) {
        click();
    } else if (!strcmp(cmd, "dblclick") && argc == 2) {
        click(); nap(60); click();
    } else if (!strcmp(cmd, "move") && argc == 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
    } else if (!strcmp(cmd, "clickat") && argc == 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        nap(120);
        click();
    } else if (!strcmp(cmd, "dblclickat") && argc == 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        nap(120);
        click(); nap(60); click();
    } else {
        die("bad subcommand or args");
    }
    printf("ok %s\n", cmd);
    return 0;
}
