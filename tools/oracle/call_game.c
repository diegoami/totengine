/* Controlled native call for the owned Wine oracle. The caller writes a
 * DWORD WINAPI(void *) thunk into the already-dead startup cave. Suspend
 * the game's GUI thread during execution so city/math state cannot race
 * an input action. The thunk must avoid presentation and Windows calls.
 * Usage: call_game.exe <game hwnd hex> <thunk address hex>
 * Build with the same llvm-mingw toolchain as winject.c. */
#include <windows.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 3) { fprintf(stderr,"usage: call_game hwnd thunk\n"); return 2; }
    HWND window = (HWND)(uintptr_t)strtoull(argv[1],NULL,16);
    DWORD pid=0, tid=GetWindowThreadProcessId(window,&pid);
    if (!tid) { fprintf(stderr,"window missing\n"); return 2; }
    HANDLE process=OpenProcess(PROCESS_CREATE_THREAD|PROCESS_QUERY_INFORMATION|PROCESS_VM_OPERATION|PROCESS_VM_READ|PROCESS_VM_WRITE,FALSE,pid);
    HANDLE gui=OpenThread(THREAD_SUSPEND_RESUME,FALSE,tid);
    if (!process || !gui) { fprintf(stderr,"open error %lu\n",GetLastError()); return 3; }
    if (SuspendThread(gui)==(DWORD)-1) { fprintf(stderr,"suspend error %lu\n",GetLastError()); return 3; }
    HANDLE call=CreateRemoteThread(process,NULL,0,(LPTHREAD_START_ROUTINE)(uintptr_t)strtoull(argv[2],NULL,16),NULL,0,NULL);
    DWORD wait=call ? WaitForSingleObject(call,30000) : WAIT_FAILED;
    DWORD result=0;
    if (wait==WAIT_OBJECT_0) GetExitCodeThread(call,&result);
    ResumeThread(gui);
    if (call) CloseHandle(call);
    CloseHandle(gui); CloseHandle(process);
    if (wait!=WAIT_OBJECT_0) { fprintf(stderr,"call wait %lu; do not restore cave until the owned game is stopped\n",wait); return 4; }
    printf("%lu\n",result);
    return 0;
}
