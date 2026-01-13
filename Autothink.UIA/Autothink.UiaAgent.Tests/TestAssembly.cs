// 说明:
// - 测试程序集级别配置：禁止并行执行，避免 UIA/剪贴板等资源竞争。
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]
