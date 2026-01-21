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
// 🔴 必填：在 Cheat Engine 中搜到的 CHWDataContainer 对象地址
static void* TARGET_CONTAINER_ADDR = (void*)0x12BA1278; 

// 函数偏移 (基于 IDA 基址 10000000)
static const uintptr_t OFFSET_GetNewID = 0x471A0;
static const uintptr_t OFFSET_Register = 0x1CF00;
static const uintptr_t OFFSET_Notify   = 0x4A380;
static const uintptr_t OFFSET_Link     = 0x51AA0;

// 关键 this 指针偏移 (基于你的汇编截图确凿证据)
static const uintptr_t THIS_OFFSET_REGISTER = 0x08;   // "add ecx, 8"
static const uintptr_t THIS_OFFSET_NOTIFY   = 0x36C;  // "lea ecx, [esi+36Ch]"
static const uintptr_t THIS_OFFSET_LINK_P2C = 0x3A4;  // "lea ecx, [esi+3A4h]" (父->子)
static const uintptr_t THIS_OFFSET_LINK_C2P = 0x3C0;  // "lea ecx, [esi+3C0h]" (子->父)

// 假设父节点 ID 为 1 (Hardware Root)
static const int PARENT_ID = 1;

const TCHAR* PAYLOAD_PATH = _T("C:\\payload.bin");

// ==========================================================================
// CORE LOGIC
// ==========================================================================

class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

// Wrappers
__declspec(naked) void ASM_Call_GetNewID(void* fn, void* pThis, void* pSlave) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]
        push [ebp+16]
        call [ebp+8]
        pop ebp
        ret
    }
}

__declspec(naked) void** ASM_Call_Register(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]
        push [ebp+16]
        call [ebp+8]
        pop ebp
        ret
    }
}

__declspec(naked) void ASM_Call_Notify(void* fn, void* pThis, int* pIdPtr) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]
        push [ebp+16]
        call [ebp+8]
        pop ebp
        ret
    }
}

__declspec(naked) int* ASM_Call_Link(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]
        push [ebp+16]
        call [ebp+8]
        pop ebp
        ret
    }
}

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    void* pContainer = TARGET_CONTAINER_ADDR;
    if (IsBadReadPtr(pContainer, 4)) {
        ::MessageBox(NULL, _T("Address Invalid"), 0, 0); return;
    }

    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    DWORD_PTR base = (DWORD_PTR)hLogic;

    void* fnGetNewID = (void*)(base + OFFSET_GetNewID);
    void* fnRegister = (void*)(base + OFFSET_Register);
    void* fnNotify   = (void*)(base + OFFSET_Notify);
    void* fnLink     = (void*)(base + OFFSET_Link);

    // Load Payload
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
    CObject* pSlave = pfnGetClass()->CreateObject();
    try { pSlave->Serialize(ar); } catch(...) { delete pSlave; delete[] buf; return; }
    ar.Close(); delete[] buf;

    // Injection Sequence
    try {
        // 1. Allocate ID
        ASM_Call_GetNewID(fnGetNewID, pContainer, pSlave);
        int new_id = *((int*)((char*)pSlave + 24));

        // 2. Register (Map[id] = Object)
        void* pRegThis = (char*)pContainer + THIS_OFFSET_REGISTER; // +8
        void** pSlot = ASM_Call_Register(fnRegister, pRegThis, new_id);
        if (pSlot) *pSlot = pSlave;

        // 3. Link: Child -> Parent (关键！UI 树向上查找父节点)
        // 汇编证据: lea ecx, [esi+3C0h]
        void* pLinkC2P = (char*)pContainer + THIS_OFFSET_LINK_C2P; // +0x3C0
        int* pSlotC2P = ASM_Call_Link(fnLink, pLinkC2P, new_id);
        if (pSlotC2P) *pSlotC2P = PARENT_ID; 

        // 4. Link: Parent -> Child (反向查找，有的逻辑需要)
        // 汇编证据: lea ecx, [esi+3A4h]
        // 注意：这里参数 Key 是 ParentID，Value 是 ChildID
        void* pLinkP2C = (char*)pContainer + THIS_OFFSET_LINK_P2C; // +0x3A4
        int* pSlotP2C = ASM_Call_Link(fnLink, pLinkP2C, PARENT_ID);
        if (pSlotP2C) *pSlotP2C = new_id;

        // 5. Notify UI
        void* pNotifyThis = (char*)pContainer + THIS_OFFSET_NOTIFY; // +0x36C
        ASM_Call_Notify(fnNotify, pNotifyThis, &new_id);

        ::MessageBox(NULL, _T("✅ INJECTION DONE!\nCheck Tree View."), _T("Victory"), MB_OK);
    }
    catch (...) {
        ::MessageBox(NULL, _T("Crash in Logic"), _T("Error"), MB_OK);
    }
}