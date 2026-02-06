// Header for doomgeneric syscalls
#ifndef DOOMGENERIC_SYSCALLS_H
#define DOOMGENERIC_SYSCALLS_H

void DG_Sys_Framebuffer(void* fb, int w, int h);
int DG_Sys_ReadKey(void);
int DG_Sys_OpenWAD(const char* path);
int DG_Sys_ReadWAD(int fd, void* buf, int len);

#endif // DOOMGENERIC_SYSCALLS_H
