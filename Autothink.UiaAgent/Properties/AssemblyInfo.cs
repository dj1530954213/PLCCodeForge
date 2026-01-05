// 说明:
// - 允许测试项目访问 internal 成员，便于覆盖内部解析/匹配逻辑。
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("Autothink.UiaAgent.Tests")]
