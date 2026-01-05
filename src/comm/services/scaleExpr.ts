export type CompiledScaleExpr =
  | { ok: true; apply: (x: number) => number }
  | { ok: false; message: string };

type ParseResult<T> = { ok: true; value: T } | { ok: false; message: string };

type ScaleOp = "+" | "-" | "*" | "/" | "u-" | "u+";

type Token =
  | { kind: "number"; value: number }
  | { kind: "var" }
  | { kind: "op"; op: ScaleOp }
  | { kind: "lparen" }
  | { kind: "rparen" };

const PRECEDENCE: Record<ScaleOp, number> = { "u-": 3, "u+": 3, "*": 2, "/": 2, "+": 1, "-": 1 };

function isRightAssociative(op: ScaleOp): boolean {
  return op === "u-" || op === "u+";
}

function replaceAllCompat(raw: string, search: string, replacement: string): string {
  return raw.split(search).join(replacement);
}

function tokenize(inputRaw: string): ParseResult<Token[]> {
  const input = inputRaw.trim();
  if (!input) return { ok: false, message: "表达式不能为空" };

  // Only support {{x}} placeholder. Replace with variable `x`.
  const replaced = replaceAllCompat(input, "{{x}}", "x");
  if (/[{}]/.test(replaced)) {
    return { ok: false, message: "仅支持 {{x}} 占位符" };
  }

  const tokens: Token[] = [];
  let i = 0;
  while (i < replaced.length) {
    const ch = replaced[i];
    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      i += 1;
      continue;
    }
    if (ch === "(") {
      tokens.push({ kind: "lparen" });
      i += 1;
      continue;
    }
    if (ch === ")") {
      tokens.push({ kind: "rparen" });
      i += 1;
      continue;
    }
    if (ch === "+" || ch === "-" || ch === "*" || ch === "/") {
      tokens.push({ kind: "op", op: ch });
      i += 1;
      continue;
    }
    if (ch === "x") {
      tokens.push({ kind: "var" });
      i += 1;
      continue;
    }
    if ((ch >= "0" && ch <= "9") || ch === ".") {
      let j = i + 1;
      while (j < replaced.length) {
        const cj = replaced[j];
        if ((cj >= "0" && cj <= "9") || cj === ".") {
          j += 1;
          continue;
        }
        break;
      }
      const raw = replaced.slice(i, j);
      const n = Number(raw);
      if (!Number.isFinite(n)) {
        return { ok: false, message: `数字解析失败: ${raw}` };
      }
      tokens.push({ kind: "number", value: n });
      i = j;
      continue;
    }

    return { ok: false, message: `不支持的字符: ${ch}` };
  }

  return { ok: true, value: tokens };
}

function toRpn(tokens: Token[]): ParseResult<Token[]> {
  const output: Token[] = [];
  const ops: Token[] = [];
  let prev: Token | null = null;

  for (const t of tokens) {
    if (t.kind === "number" || t.kind === "var") {
      output.push(t);
      prev = t;
      continue;
    }

    if (t.kind === "lparen") {
      ops.push(t);
      prev = t;
      continue;
    }

    if (t.kind === "rparen") {
      while (ops.length > 0 && ops[ops.length - 1].kind !== "lparen") {
        output.push(ops.pop() as Token);
      }
      if (ops.length === 0) return { ok: false, message: "括号不匹配：缺少 (" };
      ops.pop(); // pop '('
      prev = t;
      continue;
    }

    if (t.kind === "op") {
      const isUnary: boolean =
        (t.op === "+" || t.op === "-") &&
        (prev === null ||
          prev.kind === "op" ||
          prev.kind === "lparen");
      const op: Token = isUnary ? { kind: "op", op: t.op === "-" ? "u-" : "u+" } : t;

      while (ops.length > 0) {
        const top = ops[ops.length - 1];
        if (top.kind !== "op") break;

        const precTop = PRECEDENCE[top.op];
        const precOp = PRECEDENCE[op.op];
        const shouldPop =
          isRightAssociative(op.op) ? precTop > precOp : precTop >= precOp;
        if (!shouldPop) break;
        output.push(ops.pop() as Token);
      }
      ops.push(op);
      prev = op;
      continue;
    }
  }

  while (ops.length > 0) {
    const t = ops.pop() as Token;
    if (t.kind === "lparen" || t.kind === "rparen") {
      return { ok: false, message: "括号不匹配" };
    }
    output.push(t);
  }

  return { ok: true, value: output };
}

function evalRpn(rpn: Token[], x: number): number {
  const stack: number[] = [];
  for (const t of rpn) {
    if (t.kind === "number") {
      stack.push(t.value);
      continue;
    }
    if (t.kind === "var") {
      stack.push(x);
      continue;
    }
    if (t.kind !== "op") {
      throw new Error("invalid rpn token");
    }

    if (t.op === "u-" || t.op === "u+") {
      const a = stack.pop();
      if (a === undefined) throw new Error("missing operand");
      stack.push(t.op === "u-" ? -a : +a);
      continue;
    }

    const b = stack.pop();
    const a = stack.pop();
    if (a === undefined || b === undefined) throw new Error("missing operand");
    let out: number;
    switch (t.op) {
      case "+":
        out = a + b;
        break;
      case "-":
        out = a - b;
        break;
      case "*":
        out = a * b;
        break;
      case "/":
        out = a / b;
        break;
      default:
        throw new Error(`unknown op: ${t.op}`);
    }
    stack.push(out);
  }

  if (stack.length !== 1) throw new Error("invalid expression");
  return stack[0];
}

export function compileScaleExpr(exprRaw: string): CompiledScaleExpr {
  const tok = tokenize(exprRaw);
  if (!tok.ok) return tok;

  const rpn = toRpn(tok.value);
  if (!rpn.ok) return rpn;

  return {
    ok: true,
    apply: (x: number) => {
      const n = evalRpn(rpn.value, x);
      if (!Number.isFinite(n)) throw new Error("result is not finite");
      return n;
    },
  };
}
