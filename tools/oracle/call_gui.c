/* Invoke an owned oracle thunk on its GUI thread. Build for i686 (the game
 * is 32-bit). The thunk is DWORD WINAPI(void *), in the dead startup cave.
 * Preserve registers/flags and resume at the suspended instruction. */
#include <windows.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN64
#error Build this helper for i686-pc-windows-gnu
#endif

int main(int argc,char **argv) {
    if(argc!=3){fprintf(stderr,"usage: call_gui hwnd thunk\n");return 2;}
    HWND window=(HWND)(uintptr_t)strtoul(argv[1],NULL,16);
    DWORD pid=0,tid=GetWindowThreadProcessId(window,&pid);
    HANDLE process=OpenProcess(PROCESS_QUERY_INFORMATION|PROCESS_VM_OPERATION|PROCESS_VM_READ|PROCESS_VM_WRITE,FALSE,pid);
    HANDLE gui=OpenThread(THREAD_SUSPEND_RESUME|THREAD_GET_CONTEXT|THREAD_SET_CONTEXT,FALSE,tid);
    if(!process||!gui){fprintf(stderr,"open error %lu\n",GetLastError());return 3;}
    DWORD thunk=strtoul(argv[2],NULL,16),wrapper=thunk+448,flag=0x607c68,entry=0x607c6c;
    DWORD old_flag=0,old_entry=0,zero=0,one=1,rel;SIZE_T n=0;
    unsigned char code[31],old[31];CONTEXT context={0};context.ContextFlags=CONTEXT_CONTROL;
    if(SuspendThread(gui)==(DWORD)-1){return 3;}
    if(!GetThreadContext(gui,&context)
        || !ReadProcessMemory(process,(void *)(uintptr_t)wrapper,old,sizeof(old),&n)
        || !ReadProcessMemory(process,(void *)(uintptr_t)flag,&old_flag,4,&n)
        || !ReadProcessMemory(process,(void *)(uintptr_t)entry,&old_entry,4,&n)) {ResumeThread(gui);return 3;}
    code[0]=0x9c;code[1]=0x60;code[2]=0xc7;code[3]=0x05;
    memcpy(code+4,&entry,4);memcpy(code+8,&one,4);
    code[12]=0x6a;code[13]=0;code[14]=0xe8;rel=thunk-(wrapper+19);memcpy(code+15,&rel,4);
    code[19]=0xa3;memcpy(code+20,&flag,4);code[24]=0x61;code[25]=0x9d;code[26]=0xe9;
    rel=context.Eip-(wrapper+31);memcpy(code+27,&rel,4);
    if(!WriteProcessMemory(process,(void *)(uintptr_t)wrapper,code,sizeof(code),&n)
        || !WriteProcessMemory(process,(void *)(uintptr_t)flag,&zero,4,&n)
        || !WriteProcessMemory(process,(void *)(uintptr_t)entry,&zero,4,&n)) {ResumeThread(gui);return 3;}
    context.Eip=wrapper;
    if(!SetThreadContext(gui,&context)){ResumeThread(gui);return 3;}
    ResumeThread(gui);PostMessageA(window,WM_NULL,0,0);
    DWORD start=GetTickCount(),result=0;int done=0;
    while(GetTickCount()-start<30000) {
        ReadProcessMemory(process,(void *)(uintptr_t)flag,&result,4,&n);
        if(result) {
            SuspendThread(gui);CONTEXT current={0};current.ContextFlags=CONTEXT_CONTROL;
            if(GetThreadContext(gui,&current) && (current.Eip<thunk||current.Eip>=thunk+512)) {
                done=WriteProcessMemory(process,(void *)(uintptr_t)wrapper,old,sizeof(old),&n)
                    && WriteProcessMemory(process,(void *)(uintptr_t)flag,&old_flag,4,&n)
                    && WriteProcessMemory(process,(void *)(uintptr_t)entry,&old_entry,4,&n);
            }
            ResumeThread(gui);if(done)break;
        }
        if((GetTickCount()-start)%100==0)PostMessageA(window,WM_NULL,0,0);
        Sleep(1);
    }
    if(!done) {
        DWORD entered=0;ReadProcessMemory(process,(void *)(uintptr_t)entry,&entered,4,&n);
        SuspendThread(gui);CONTEXT current={0};current.ContextFlags=CONTEXT_CONTROL;
        GetThreadContext(gui,&current);ResumeThread(gui);
        fprintf(stderr,"GUI call timed out: entered=%lu result=%lu EIP=%08lx; stop owned game before restoring cave\n",entered,result,current.Eip);
    }
    CloseHandle(gui);CloseHandle(process);
    if(!done)return 4;
    printf("%lu\n",result);return 0;
}
