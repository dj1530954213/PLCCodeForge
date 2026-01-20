#include <afx.h>
#include <afxwin.h>
#include <windows.h>
#include <psapi.h> // 用于获取 DLL 内存范围

#pragma comment(lib, "Psapi.lib")

// 存根类定义
class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
};

// 辅助函数：获取 dllDPLogic.dll 的内存起始和结束地址
static bool GetModuleRange(HMODULE h, uintptr_t& base, uintptr_t& end) {
    MODULEINFO mi{};
    if (!GetModuleInformation(GetCurrentProcess(), h, &mi, sizeof(mi))) return false;
    base = (uintptr_t)mi.lpBaseOfDll;
    end  = base + mi.SizeOfImage;
    return true;
}

// 安全读取指针：避免访问非法地址导致崩溃
static bool TryReadPtr(void* addr, void** out) {
    if (!addr || !out) return false;
    SIZE_T bytes_read = 0;
    return ReadProcessMemory(GetCurrentProcess(), addr, out, sizeof(void*), &bytes_read)
        && bytes_read == sizeof(void*);
}

// 安全调用 Serialize：捕获访问违规
static bool TrySerialize(CObject* obj, CArchive* ar) {
    if (!obj || !ar) return false;
    __try {
        obj->Serialize(*ar);
        return true;
    }
    __except(EXCEPTION_EXECUTE_HANDLER) {
        return false;
    }
}

// 辅助函数：校验单个对象的 VTable 是否合法
// 返回 true 表示 VTable 在 dllDPLogic 范围内
static bool CheckAndReport(const char* tag, void* obj, uintptr_t base, uintptr_t end) {
    if (!obj) return false;

    void* vtbl = nullptr;
    bool in_range = false;

    if (!TryReadPtr(obj, &vtbl)) {
        CString err;
        err.Format(_T("[%s] Error: Cannot read memory at %p (AV)"), CString(tag), obj);
        MessageBox(NULL, err, _T("Check Fail"), MB_OK | MB_ICONERROR);
        return false;
    }

    if ((uintptr_t)vtbl >= base && (uintptr_t)vtbl < end) {
        in_range = true;
    }

    // 弹窗报告校验结果
    CString msg;
    msg.Format(_T("[%s] Report:\nAddress: %p\nVTable: %p\nDLL Range: %p - %p\n\nIN_RANGE = %s"), 
        CString(tag), obj, vtbl, (void*)base, (void*)end, 
        in_range ? _T("YES (Valid Object)") : _T("NO (Invalid/Wrong DLL)"));
    
    MessageBox(NULL, msg, _T("VTable Verification"), MB_OK);
    return in_range;
}

// 辅助函数：带 SEH 保护的安全导出
void SafeDump(void* pRealObj, const char* filename) {
    CFile f;
    if (!f.Open(filename, CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
        MessageBox(NULL, _T("Failed to create dump file"), _T("File Error"), MB_OK);
        return;
    }

    CArchive ar(&f, CArchive::store);
    bool ok = TrySerialize((CObject*)pRealObj, &ar);

    // 清理资源
    try {
        ar.Close();
        f.Close();
    } catch(...) {}

    if (!ok) {
        MessageBox(NULL, _T("Result: SEH Crash inside Serialize\n(Access Violation occurred)"), _T("DUMP RESULT"), MB_OK | MB_ICONERROR);
    } else {
        CString msg;
        msg.Format(_T("Result: Dump Success!\nSaved to: %s"), CString(filename));
        MessageBox(NULL, msg, _T("DUMP RESULT"), MB_OK);
    }
}

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 1. 获取 DLL 范围
    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) { 
        MessageBox(NULL, _T("dllDPLogic.dll not loaded"), _T("Error"), MB_OK); 
        return; 
    }

    uintptr_t base = 0, end = 0;
    if (!GetModuleRange(hLogic, base, end)) return;

    // 2. 定义两个候选者
    // candA: 直接把该地址当对象
    void* candA = (void*)0x0F2D2A04; 
    
    // candB: 把该地址当指针，取其指向的内容
    void* candB = nullptr;
    if (!IsBadReadPtr(candA, 4)) {
        TryReadPtr(candA, &candB);
    }

    // 3. 执行校验 (Step A)
    // 两个都会弹窗报告 IN_RANGE = YES 或 NO
    bool a_ok = CheckAndReport("candA (Direct)", candA, base, end);
    
    bool b_ok = false;
    if (candB) {
        b_ok = CheckAndReport("candB (Dereferenced)", candB, base, end);
    }

    // 4. 根据结果执行导出 (Step B)
    if (a_ok) {
        if (MessageBox(NULL, _T("candA looks VALID. Attempt Dump?"), _T("Next Step"), MB_YESNO) == IDYES) {
            SafeDump(candA, "C:\\valid_dump.bin");
        }
    } 
    else if (b_ok) {
        if (MessageBox(NULL, _T("candB looks VALID. Attempt Dump?"), _T("Next Step"), MB_YESNO) == IDYES) {
            SafeDump(candB, "C:\\valid_dump.bin");
        }
    } 
    else {
        MessageBox(NULL, _T("Neither candidate is valid.\nCannot proceed to dump."), _T("Conclusion"), MB_OK | MB_ICONWARNING);
    }
}
