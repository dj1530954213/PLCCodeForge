#include <afx.h>
#include <afxwin.h>
#include <vector>
#include <fstream>
#include <iomanip>

#define IDA_BASE 0x00400000
#define TCP_MANAGER_IDA_ADDR 0x0084713C

void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Dump Tool Error"), MB_OK | MB_ICONERROR);
}

extern "C" __declspec(dllexport) void RunPoc()
{
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    DWORD_PTR baseAddr = (DWORD_PTR)GetModuleHandle(NULL);
    DWORD_PTR offset = TCP_MANAGER_IDA_ADDR - IDA_BASE;
    void** pManagerPtrLoc = (void**)(baseAddr + offset);

    if (IsBadReadPtr(pManagerPtrLoc, 4)) {
        ShowError(_T("Calculated global variable address is invalid."));
        return;
    }

    void* pManager = *pManagerPtrLoc;

    if (!pManager) {
        ShowError(_T("Manager pointer is NULL.\nPlease open a project and click the tree view first."));
        return;
    }
    if (IsBadReadPtr(pManager, 4)) {
        ShowError(_T("Manager points to invalid memory (Garbage data)."));
        return;
    }

    const char* dumpPath = "C:\\manager_dump.txt";
    std::ofstream dumpFile(dumpPath);
    if (!dumpFile.is_open()) {
        ShowError(_T("Failed to create C:\\manager_dump.txt.\nCheck permissions or if file is open."));
        return;
    }

    unsigned char* pMem = (unsigned char*)pManager;

    dumpFile << "TCP Manager Memory Dump\n";
    dumpFile << "=======================\n";
    dumpFile << "Instance Address: " << std::hex << pManager << "\n";
    dumpFile << "Pointer Located At: " << std::hex << pManagerPtrLoc << "\n";
    dumpFile << "Target Device Count: 5 (Look for '05 00 00 00')\n\n";

    dumpFile << "Offset | 00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F | ASCII Representation\n";
    dumpFile << "-------|-------------------------------------------------|---------------------\n";

    for (int i = 0; i < 512; i += 16) {
        dumpFile << std::hex << std::setw(4) << std::setfill('0') << i << " | ";

        for (int j = 0; j < 16; j++) {
            if (IsBadReadPtr(pMem + i + j, 1)) {
                dumpFile << "?? ";
            } else {
                dumpFile << std::hex << std::setw(2) << std::setfill('0') << (int)pMem[i + j] << " ";
            }
        }

        dumpFile << "| ";
        for (int j = 0; j < 16; j++) {
            if (IsBadReadPtr(pMem + i + j, 1)) {
                dumpFile << ".";
            } else {
                char c = pMem[i + j];
                dumpFile << ((c >= 32 && c <= 126) ? c : '.');
            }
        }
        dumpFile << "\n";
    }

    dumpFile.close();

    CString successMsg;
    successMsg.Format(_T("✅ Dump Success!\n\nFile saved to: %S\n\nPlease copy the content of this text file to AI."), dumpPath);
    ::MessageBox(NULL, successMsg, _T("Snapshot Taken"), MB_OK);
}
