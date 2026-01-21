// #include <afx.h>
// #include <afxwin.h>

// class CModbusSlave : public CObject {
// public:
//     virtual void Serialize(CArchive& ar);
//     static CRuntimeClass* PASCAL GetThisClass();
// };

// extern "C" __declspec(dllexport) void RunPoc() {
//     AFX_MANAGE_STATE(AfxGetStaticModuleState());

//     // 1. 读取 Payload
//     CFile fLoad;
//     CFileException e;
//     if (!fLoad.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary, &e)) {
//         ::MessageBox(NULL, _T("Payload not found"), 0, 0);
//         return;
//     }

//     HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
//     if (!hLogic) return;
//     typedef CRuntimeClass* (*FnGetClass)();
//     FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
//     if (!pfnGetClass) return;

//     CObject* pObj = pfnGetClass()->CreateObject();

//     // 2. Load (反序列化)
//     CArchive arLoad(&fLoad, CArchive::load);
//     try {
//         pObj->Serialize(arLoad);
//     }
//     catch(...) {
//         ::MessageBox(NULL, _T("Load Failed!"), _T("Error"), MB_OK);
//         delete pObj;
//         return;
//     }
//     arLoad.Close();
//     fLoad.Close();

//     // 3. Round-Trip Store (再次序列化到新文件)
//     CFile fStore;
//     if (fStore.Open(_T("C:\\roundtrip.bin"), CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
//         CArchive arStore(&fStore, CArchive::store);
//         try {
//             pObj->Serialize(arStore);
//             ::MessageBox(NULL, _T("Round-Trip Success!\nCheck C:\\roundtrip.bin"), _T("Victory"), MB_OK);
//         }
//         catch(...) {
//             ::MessageBox(NULL, _T("Round-Trip Store Failed"), _T("Error"), MB_OK);
//         }
//         arStore.Close();
//         fStore.Close();
//     }

//     delete pObj;
// }

#include <afx.h>
#include <afxwin.h>

// ==========================================================================
// CONFIGURATION ZONE
// ==========================================================================
// 🔴 必填：在 Cheat Engine 中搜到的 CHWDataContainer 对象地址 (Hex 632F7FE0 的持有者)
static void* TARGET_CONTAINER_ADDR = (void*)0x125DE338; 

// ==========================================================================
// CONSTANTS (基于 IDA 汇编确凿证据)
// ==========================================================================
static const uintptr_t OFFSET_GetNewID = 0x471A0;
static const uintptr_t OFFSET_Register = 0x1CF00;
static const uintptr_t OFFSET_Notify   = 0x4A380;

// 关键偏移修正 (Key Fixes from Assembly Analysis)
static const uintptr_t THIS_OFFSET_REGISTER = 0x08;   // "add ecx, 8"
static const uintptr_t THIS_OFFSET_NOTIFY   = 0x36C;  // "lea ecx, [esi+36Ch]"

const TCHAR* PAYLOAD_PATH = _T("C:\\payload.bin");

// ==========================================================================
// CORE LOGIC
// ==========================================================================

class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

// Assembly Wrappers (Force __thiscall)
__declspec(naked) void ASM_Call_GetNewID(void* fn, void* pThis, void* pSlave) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis -> ECX
        push [ebp+16]       // pSlave -> Stack
        call [ebp+8]        // Call fn
        pop ebp
        ret
    }
}

__declspec(naked) void** ASM_Call_Register(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis -> ECX
        push [ebp+16]       // id -> Stack
        call [ebp+8]        // Call fn (Result in EAX)
        pop ebp
        ret
    }
}

__declspec(naked) void ASM_Call_Notify(void* fn, void* pThis, int* pIdPtr) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis -> ECX
        push [ebp+16]       // pIdPtr -> Stack
        call [ebp+8]        // Call fn
        pop ebp
        ret
    }
}

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 1. 环境校验
    void* pContainer = TARGET_CONTAINER_ADDR;
    if (IsBadReadPtr(pContainer, 4)) {
        ::MessageBox(NULL, _T("Target Address Invalid! Update Source Code."), _T("Error"), MB_OK);
        return;
    }

    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) { ::MessageBox(NULL, _T("DLL not loaded"), 0, 0); return; }
    DWORD_PTR base = (DWORD_PTR)hLogic;

    void* fnGetNewID = (void*)(base + OFFSET_GetNewID);
    void* fnRegister = (void*)(base + OFFSET_Register);
    void* fnNotify   = (void*)(base + OFFSET_Notify);

    // 2. 加载 Payload
    CFile f;
    if (!f.Open(PAYLOAD_PATH, CFile::modeRead | CFile::typeBinary)) {
        ::MessageBox(NULL, _T("Payload missing"), 0, 0); return;
    }
    ULONGLONG len = f.GetLength();
    BYTE* buf = new BYTE[(size_t)len];
    f.Read(buf, (UINT)len);
    f.Close();
    CMemFile mem(buf, (UINT)len);
    CArchive ar(&mem, CArchive::load);

    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!pfnGetClass) { delete[] buf; return; }

    CObject* pSlave = pfnGetClass()->CreateObject();
    try { pSlave->Serialize(ar); } 
    catch(...) { delete pSlave; delete[] buf; return; }
    ar.Close(); delete[] buf;

    // 3. 执行挂载 (Atomic Injection with Correct Offsets)
    try {
        // Step A: 分配 ID
        // GetNewID 使用基地址 pContainer
        ASM_Call_GetNewID(fnGetNewID, pContainer, pSlave);

        // Step B: 获取 ID
        int id = *((int*)((char*)pSlave + 24));

        // Step C: 存入哈希表 (修正偏移 +0x08)
        void* pMapThis = (char*)pContainer + THIS_OFFSET_REGISTER; 
        void** pSlot = ASM_Call_Register(fnRegister, pMapThis, id);
        
        if (pSlot) {
            *pSlot = pSlave;
        } else {
            ::MessageBox(NULL, _T("Register failed"), 0, 0); return;
        }

        // Step D: 通知刷新 (修正偏移 +0x36C)
        // 关键修正：这里必须使用偏移后的地址！
        void* pNotifyThis = (char*)pContainer + THIS_OFFSET_NOTIFY;
        ASM_Call_Notify(fnNotify, pNotifyThis, &id);

        ::MessageBox(NULL, _T("✅ INJECTION SUCCESS!\nCheck Tree View."), _T("VICTORY"), MB_OK);
    }
    catch (...) {
        ::MessageBox(NULL, _T("Crash in Mount logic"), _T("Fatal"), MB_OK);
    }
}