#define _CRT_SECURE_NO_WARNINGS
#include <afx.h>
#include <afxwin.h>
#include <iostream>
#include <iomanip>

// ==========================================================================
// ⚙️ 偏移量配置 (基于 IDA 基址 10000000)
// ==========================================================================
const TCHAR* PAYLOAD_PATH = _T("C:\\payload.bin");

static const uintptr_t OFFSET_SafeLookup = 0xB4F0;
static const uintptr_t OFFSET_GetNewID   = 0x471A0;
static const uintptr_t OFFSET_Register   = 0x1CF00;
static const uintptr_t OFFSET_Notify     = 0x4A380;
static const uintptr_t OFFSET_Link       = 0x51AA0;

static const uintptr_t THIS_OFFSET_REGISTER = 0x08;
static const uintptr_t THIS_OFFSET_NOTIFY   = 0x36C;
static const uintptr_t THIS_OFFSET_LINK_C2P = 0x3C0;
static const uintptr_t THIS_OFFSET_LINK_P2C = 0x3A4;

// ==========================================================================
// 🔧 汇编包装器 (已展开为标准多行格式，修复 C2601/C1075 错误)
// ==========================================================================

__declspec(naked) void ASM_Call_GetNewID(void* fn, void* pThis, void* pSlave) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis
        push [ebp+16]       // pSlave
        call [ebp+8]        // fn
        pop ebp
        ret
    }
}

__declspec(naked) void** ASM_Call_Register(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis
        push [ebp+16]       // id
        call [ebp+8]        // fn
        pop ebp
        ret
    }
}

__declspec(naked) void ASM_Call_Notify(void* fn, void* pThis, int* pIdPtr) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis
        push [ebp+16]       // pIdPtr
        call [ebp+8]        // fn
        pop ebp
        ret
    }
}

__declspec(naked) int* ASM_Call_Link(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis
        push [ebp+16]       // id
        call [ebp+8]        // fn
        pop ebp
        ret
    }
}

class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

// ==========================================================================
// 🛠️ 辅助功能：打印 VTable
// ==========================================================================
void PrintSlaveVTable(HMODULE hLogic) {
    typedef CRuntimeClass* (*Fn)();
    Fn GetClass = (Fn)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!GetClass) {
        std::cout << "[-] Error: Cannot find CModbusSlave factory." << std::endl;
        return;
    }
    
    CObject* p = GetClass()->CreateObject();
    unsigned int vtbl = *(unsigned int*)p;
    
    std::cout << "\n------------------------------------------------\n";
    std::cout << " [STEP 1] Find Existing Slave ID\n";
    std::cout << "------------------------------------------------\n";
    std::cout << "Target VTable (Hex): " << std::hex << std::uppercase << vtbl << std::dec << "\n";
    std::cout << "Action:\n";
    std::cout << "  1. Search this HEX value in Cheat Engine (4 Bytes).\n";
    std::cout << "  2. Pick any result address (NOT static/green ones).\n";
    std::cout << "  3. Look at offset +24 (0x18). That number is the ID.\n";
    
    delete p;
}

// ==========================================================================
// 🚀 主流程
// ==========================================================================
extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());
    
    AllocConsole();
    freopen("CONIN$", "r", stdin);
    freopen("CONOUT$", "w", stdout);
    freopen("CONOUT$", "w", stderr);

    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) { std::cout << "[-] DLL not loaded." << std::endl; return; }
    DWORD_PTR base = (DWORD_PTR)hLogic;

    // 1. 打印 VTable 供你查找 Slave ID
    PrintSlaveVTable(hLogic);

    // 2. 等待输入
    uintptr_t addrInput = 0;
    int slaveID = 0;

    std::cout << "\n------------------------------------------------\n";
    std::cout << " [STEP 2] Input Data\n";
    std::cout << "------------------------------------------------\n";
    std::cout << "Enter Container Address (Hex) [Your 143004E0]: ";
    std::cin >> std::hex >> addrInput;
    
    std::cout << "Enter Existing Slave ID (Dec) [Found in CE]: ";
    std::cin >> std::dec >> slaveID;

    void* pContainer = (void*)addrInput;
    if (IsBadReadPtr(pContainer, 4)) {
        std::cout << "[-] Invalid Container Address." << std::endl; return;
    }

    // 3. 自动反查 Parent (侦探逻辑)
    std::cout << "\n------------------------------------------------\n";
    std::cout << " [STEP 3] Detect Parent & Inject\n";
    std::cout << "------------------------------------------------\n";
    
    int parentID = 0;
    void* fnLink = (void*)(base + OFFSET_Link);
    void* pLinkC2P = (char*)pContainer + THIS_OFFSET_LINK_C2P;

    try {
        int* pResult = ASM_Call_Link(fnLink, pLinkC2P, slaveID);
        if (pResult && !IsBadReadPtr(pResult, 4)) {
            parentID = *pResult;
            std::cout << "[+] Found Parent ID: " << parentID << std::endl;
        } else {
            std::cout << "[-] Failed to find Parent ID. Is Slave ID correct?" << std::endl;
            return;
        }
    } catch(...) { std::cout << "[-] Crash in detection." << std::endl; return; }

    // 4. 加载 Payload 并 注入
    CFile f;
    if (!f.Open(PAYLOAD_PATH, CFile::modeRead | CFile::typeBinary)) {
        std::cout << "[-] Payload not found." << std::endl; return;
    }
    ULONGLONG len = f.GetLength();
    BYTE* buf = new BYTE[(size_t)len];
    f.Read(buf, (UINT)len);
    f.Close();
    CMemFile mem(buf, (UINT)len);
    CArchive ar(&mem, CArchive::load);

    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    CObject* pSlave = pfnGetClass()->CreateObject();
    try { pSlave->Serialize(ar); } catch(...) { delete pSlave; delete[] buf; return; }
    ar.Close(); delete[] buf;

    // 准备注入函数
    void* fnGetNewID = (void*)(base + OFFSET_GetNewID);
    void* fnRegister = (void*)(base + OFFSET_Register);
    void* fnNotify   = (void*)(base + OFFSET_Notify);

    try {
        // A. Get ID
        ASM_Call_GetNewID(fnGetNewID, pContainer, pSlave);
        int new_id = *((int*)((char*)pSlave + 24));
        std::cout << "-> Allocated New ID: " << new_id << std::endl;

        // B. Register
        void* pRegThis = (char*)pContainer + THIS_OFFSET_REGISTER;
        void** pSlot = ASM_Call_Register(fnRegister, pRegThis, new_id);
        if (pSlot) *pSlot = pSlave;

        // C. Link (双向)
        void* pLinkP2C = (char*)pContainer + THIS_OFFSET_LINK_P2C; // +3A4
        int* pSlotP2C = ASM_Call_Link(fnLink, pLinkP2C, parentID);
        if (pSlotP2C) *pSlotP2C = new_id;

        int* pSlotC2P = ASM_Call_Link(fnLink, pLinkC2P, new_id);
        if (pSlotC2P) *pSlotC2P = parentID;

        // D. Notify
        void* pNotifyThis = (char*)pContainer + THIS_OFFSET_NOTIFY;
        ASM_Call_Notify(fnNotify, pNotifyThis, &new_id);

        std::cout << "\n[+] SUCCESS! Tree View Updated." << std::endl;
    }
    catch (...) {
        std::cout << "[-] Injection Crashed." << std::endl;
    }
}