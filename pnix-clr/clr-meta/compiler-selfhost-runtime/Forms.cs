using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace Pnix.ClrMeta.CompilerSupport;

public sealed class CompilerRejectionException : Exception
{
    public CompilerRejectionException(string phase, string code, object? evidence)
        : base($"compiler rejection [{phase}/{code}]: {FormDisplay.Render(evidence)}")
    {
        Phase = phase;
        Code = code;
        Evidence = evidence;
    }

    public string Phase { get; }
    public string Code { get; }
    public object? Evidence { get; }
}

public sealed class SymbolForm
{
    public SymbolForm(string name) => Name = name;
    public string Name { get; }
    public override string ToString() => Name;
}

public sealed class SequenceForm
{
    public SequenceForm(bool vector, object?[] items)
    {
        Vector = vector;
        Items = items;
    }

    public bool Vector { get; }
    public object?[] Items { get; }
}

internal sealed class PersistentEnv
{
    internal PersistentEnv(PersistentEnv? parent, string? name, object? binding)
    {
        Parent = parent;
        Name = name;
        Binding = binding;
    }

    internal PersistentEnv? Parent { get; }
    internal string? Name { get; }
    internal object? Binding { get; }

    internal object? Lookup(string name)
    {
        for (PersistentEnv? cursor = this; cursor is not null; cursor = cursor.Parent)
        {
            if (cursor.Name is not null && string.Equals(cursor.Name, name, StringComparison.Ordinal))
            {
                return cursor.Binding;
            }
        }

        return null;
    }

    internal string? LookupBindingKind(string name)
    {
        object? binding = Lookup(name);
        return binding is object?[] { Length: 3 } row && row[0] is string kind
            ? kind
            : null;
    }
}

internal static class FormDisplay
{
    internal static string Render(object? value)
    {
        return value switch
        {
            null => "nil",
            bool flag => flag ? "true" : "false",
            long number => number.ToString(CultureInfo.InvariantCulture),
            string text => $"\"{text.Replace("\\", "\\\\", StringComparison.Ordinal).Replace("\"", "\\\"", StringComparison.Ordinal)}\"",
            SymbolForm symbol => symbol.Name,
            SequenceForm sequence => RenderSequence(sequence),
            object?[] array => "[" + string.Join(" ", Array.ConvertAll(array, Render)) + "]",
            _ => value.GetType().FullName ?? value.GetType().Name,
        };
    }

    private static string RenderSequence(SequenceForm sequence)
    {
        string open = sequence.Vector ? "[" : "(";
        string close = sequence.Vector ? "]" : ")";
        return open + string.Join(" ", Array.ConvertAll(sequence.Items, Render)) + close;
    }
}

public static class ReaderAbi
{
    public const string LimitsId = "pnix.clr-meta.compiler-kernel-source-limits.v1";
    private const int MaxSourceBytes = 65_536;
    private const int MaxTopLevelForms = 37;
    private const int MaxNodes = 8_192;
    private const int MaxDepth = 128;
    private const int MaxParameters = 5;
    private const int MaxBindings = 64;

    public static object ReadAll(object? sourceValue, object? limitsIdValue)
    {
        if (sourceValue is not string source)
        {
            throw Reject("reader", "source-not-string", sourceValue);
        }

        if (limitsIdValue is not string limitsId || !string.Equals(limitsId, LimitsId, StringComparison.Ordinal))
        {
            throw Reject("reader", "source-limits-id", limitsIdValue);
        }

        int byteCount = new UTF8Encoding(false, true).GetByteCount(source);
        if (byteCount > MaxSourceBytes)
        {
            throw Reject("reader", "source-byte-budget", byteCount);
        }

        var parser = new Parser(source);
        var forms = new List<object?>();
        while (parser.SkipTrivia())
        {
            if (forms.Count == MaxTopLevelForms)
            {
                throw Reject("reader", "top-level-form-budget", forms.Count + 1L);
            }

            forms.Add(parser.ReadForm(1));
        }

        if (forms.Count == 0)
        {
            throw Reject("reader", "empty-source", null);
        }

        var result = new SequenceForm(true, forms.ToArray());
        ValidateStructuralBudgets(result);
        return result;
    }

    public static string ReadStrictUtf8File(string path)
    {
        string full = Path.GetFullPath(path);
        if (!File.Exists(full))
        {
            throw Reject("reader", "missing-source-file", full);
        }

        if ((File.GetAttributes(full) & FileAttributes.ReparsePoint) != 0)
        {
            throw Reject("reader", "source-file-reparse-point", full);
        }

        byte[] bytes;
        using (var stream = new FileStream(
            full,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            4_096,
            FileOptions.SequentialScan))
        {
            long length = stream.Length;
            if (length > MaxSourceBytes)
            {
                throw Reject("reader", "source-byte-budget", length);
            }

            bytes = new byte[checked((int)length)];
            int offset = 0;
            while (offset < bytes.Length)
            {
                int read = stream.Read(bytes, offset, bytes.Length - offset);
                if (read == 0)
                {
                    Array.Resize(ref bytes, offset);
                    break;
                }

                offset += read;
            }

            if (stream.ReadByte() != -1)
            {
                throw Reject("reader", "source-file-changed-during-read", full);
            }
        }

        try
        {
            return new UTF8Encoding(false, true).GetString(bytes);
        }
        catch (DecoderFallbackException)
        {
            throw Reject("reader", "invalid-source-utf8", full);
        }
    }

    private static CompilerRejectionException Reject(string phase, string code, object? evidence) =>
        new(phase, code, evidence);

    private static void ValidateStructuralBudgets(SequenceForm form)
    {
        if (!form.Vector && form.Items.Length > 0 && form.Items[0] is SymbolForm head)
        {
            if (string.Equals(head.Name, "let*", StringComparison.Ordinal) &&
                form.Items.Length > 1 && form.Items[1] is SequenceForm { Vector: true } bindings &&
                bindings.Items.Length > MaxBindings * 2)
            {
                throw Reject("reader", "binding-budget", bindings.Items.LongLength / 2L);
            }

            int parameterIndex = form.Items.Length > 2 && form.Items[1] is SymbolForm ? 2 : 1;
            if (string.Equals(head.Name, "fn*", StringComparison.Ordinal) &&
                form.Items.Length > parameterIndex &&
                form.Items[parameterIndex] is SequenceForm { Vector: true } parameters &&
                parameters.Items.Length > MaxParameters)
            {
                throw Reject("reader", "parameter-budget", parameters.Items.LongLength);
            }
        }

        foreach (object? item in form.Items)
        {
            if (item is SequenceForm child)
            {
                ValidateStructuralBudgets(child);
            }
        }
    }

    private sealed class Parser
    {
        private readonly string source;
        private int index;
        private int nodes;

        internal Parser(string source) => this.source = source;

        internal bool SkipTrivia()
        {
            while (index < source.Length)
            {
                char current = source[index];
                if (char.IsWhiteSpace(current))
                {
                    index++;
                    continue;
                }

                if (current == ';')
                {
                    index++;
                    while (index < source.Length && source[index] is not '\n' and not '\r')
                    {
                        index++;
                    }

                    continue;
                }

                return true;
            }

            return false;
        }

        internal object? ReadForm(int depth)
        {
            if (depth > MaxDepth)
            {
                throw Reject("reader", "source-depth-budget", depth);
            }

            SkipTrivia();
            if (index >= source.Length)
            {
                throw Reject("reader", "unexpected-eof", index);
            }

            CountNode();
            char current = source[index];
            return current switch
            {
                '(' => ReadSequence(false, ')', depth),
                '[' => ReadSequence(true, ']', depth),
                '"' => ReadString(),
                ')' or ']' => throw Reject("reader", "unexpected-closing-delimiter", current.ToString()),
                '{' or '}' => throw Reject("reader", "map-not-admitted", current.ToString()),
                '\'' or '`' or '~' or '^' or '@' or '#' or '\\' or ',' =>
                    throw Reject("reader", "reader-sugar-not-admitted", current.ToString()),
                _ => ReadToken(),
            };
        }

        private SequenceForm ReadSequence(bool vector, char closing, int depth)
        {
            index++;
            var items = new List<object?>();
            while (true)
            {
                if (!SkipTrivia())
                {
                    throw Reject("reader", "unclosed-delimiter", closing.ToString());
                }

                if (source[index] == closing)
                {
                    index++;
                    return new SequenceForm(vector, items.ToArray());
                }

                if (source[index] is ')' or ']')
                {
                    throw Reject("reader", "crossed-delimiter", source[index].ToString());
                }

                items.Add(ReadForm(depth + 1));
            }
        }

        private string ReadString()
        {
            index++;
            var builder = new StringBuilder();
            while (index < source.Length)
            {
                char current = source[index++];
                if (current == '"')
                {
                    return builder.ToString();
                }

                if (current != '\\')
                {
                    builder.Append(current);
                    continue;
                }

                if (index >= source.Length)
                {
                    throw Reject("reader", "unterminated-string-escape", null);
                }

                char escaped = source[index++];
                builder.Append(escaped switch
                {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'b' => '\b',
                    'f' => '\f',
                    'u' => ReadUnicodeEscape(),
                    _ => throw Reject("reader", "unsupported-string-escape", escaped.ToString()),
                });
            }

            throw Reject("reader", "unterminated-string", null);
        }

        private char ReadUnicodeEscape()
        {
            if (index + 4 > source.Length)
            {
                throw Reject("reader", "short-unicode-escape", null);
            }

            string digits = source.Substring(index, 4);
            index += 4;
            if (!ushort.TryParse(digits, NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out ushort value))
            {
                throw Reject("reader", "invalid-unicode-escape", digits);
            }

            return (char)value;
        }

        private object? ReadToken()
        {
            int start = index;
            while (index < source.Length && !IsTokenBoundary(source[index]))
            {
                index++;
            }

            if (start == index)
            {
                throw Reject("reader", "invalid-token-start", source[index].ToString());
            }

            string token = source.Substring(start, index - start);
            if (token[0] == ':')
            {
                throw Reject("reader", "keyword-not-admitted", token);
            }

            if (char.IsAsciiDigit(token[0]) ||
                (token.Length > 1 && (token[0] == '+' || token[0] == '-') && char.IsAsciiDigit(token[1])))
            {
                if (!LooksLikeInteger(token))
                {
                    throw Reject("reader", "invalid-numeric-token", token);
                }
            }

            return token switch
            {
                "nil" => null,
                "true" => true,
                "false" => false,
                _ when LooksLikeInteger(token) => ParseInteger(token),
                _ => new SymbolForm(token),
            };
        }

        private static bool IsTokenBoundary(char value) =>
            char.IsWhiteSpace(value) || value is '(' or ')' or '[' or ']' or '{' or '}' or '"' or ';' or ',';

        private static bool LooksLikeInteger(string token)
        {
            if (token.Length == 0)
            {
                return false;
            }

            int digit = token[0] is '+' or '-' ? 1 : 0;
            if (digit == token.Length)
            {
                return false;
            }

            for (; digit < token.Length; digit++)
            {
                if (!char.IsAsciiDigit(token[digit]))
                {
                    return false;
                }
            }

            return true;
        }

        private static long ParseInteger(string token)
        {
            if (!long.TryParse(token, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out long value))
            {
                throw Reject("reader", "int64-literal-range", token);
            }

            return value;
        }

        private void CountNode()
        {
            nodes++;
            if (nodes > MaxNodes)
            {
                throw Reject("reader", "source-node-budget", nodes);
            }
        }
    }
}

public static class DataAbi
{
    public static object KindIs(object? value, object? expectedValue)
    {
        string expected = RequireString(expectedValue, "kind");
        bool result = expected switch
        {
            "nil" => value is null,
            "boolean" => value is bool,
            "int64" => value is long,
            "string" => value is string,
            "symbol" => value is SymbolForm,
            "list" => value is SequenceForm { Vector: false },
            "vector" => value is SequenceForm { Vector: true },
            _ => false,
        };
        return result;
    }

    public static object Count(object? value)
    {
        return value switch
        {
            SequenceForm sequence => (long)sequence.Items.LongLength,
            object?[] array => (long)array.LongLength,
            _ => throw Reject("data", "count-unsupported", value),
        };
    }

    public static object? Nth(object? value, object? indexValue)
    {
        long longIndex = RequireInt64(indexValue, "index");
        if (longIndex < 0 || longIndex > int.MaxValue)
        {
            throw Reject("data", "nth-index-range", longIndex);
        }

        int index = (int)longIndex;
        object?[] items = value switch
        {
            SequenceForm sequence => sequence.Items,
            object?[] array => array,
            _ => throw Reject("data", "nth-unsupported", value),
        };
        if ((uint)index >= (uint)items.Length)
        {
            throw Reject("data", "nth-out-of-bounds", longIndex);
        }

        return items[index];
    }

    public static object SymbolName(object? value) =>
        value is SymbolForm symbol
            ? symbol.Name
            : throw Reject("data", "symbol-name-not-symbol", value);

    public static object StringEqual(object? left, object? right) =>
        left is string leftString && right is string rightString &&
        string.Equals(leftString, rightString, StringComparison.Ordinal);

    public static object EnvNew() => new PersistentEnv(null, null, null);

    public static object EnvBind(object? environment, object? nameValue, object? kindValue, object? target, object? arityValue)
    {
        PersistentEnv env = RequireEnv(environment);
        string name = RequireString(nameValue, "binding-name");
        string kind = RequireString(kindValue, "binding-kind");
        long arity = RequireInt64(arityValue, "binding-arity");
        if (kind is not ("support-call" or "intrinsic" or "kernel-call" or "constant" or "argument" or "local"))
        {
            throw Reject("validate", "binding-kind-not-admitted", kind);
        }

        if (arity < 0 || arity > 5)
        {
            throw Reject("validate", "binding-arity-range", arity);
        }

        if (kind is "kernel-call" or "constant" or "argument" or "local")
        {
            if (!IsSimpleSymbolName(name))
            {
                string code = kind switch
                {
                    "argument" => "invalid-parameter",
                    "local" => "invalid-binding-name",
                    _ => "invalid-definition-name",
                };
                throw Reject("validate", code, name);
            }
        }

        string? existingKind = env.LookupBindingKind(name);
        if (kind == "argument" && existingKind == "argument")
        {
            throw Reject("validate", "duplicate-parameter", name);
        }

        if (kind == "local" && existingKind is "argument" or "local")
        {
            throw Reject("validate", "duplicate-local-binding", name);
        }

        return new PersistentEnv(env, name, new object?[] { kind, target, arity });
    }

    public static object? EnvLookup(object? environment, object? nameValue)
    {
        PersistentEnv env = RequireEnv(environment);
        string name = RequireString(nameValue, "lookup-name");
        return env.Lookup(name);
    }

    public static object Reject(object? phaseValue, object? codeValue, object? evidence)
    {
        string phase = RequireString(phaseValue, "rejection-phase");
        string code = RequireString(codeValue, "rejection-code");
        throw Reject(phase, code, evidence);
    }

    internal static long RequireInt64(object? value, string role) =>
        value is long number ? number : throw Reject("data", role + "-not-int64", value);

    internal static bool RequireBoolean(object? value, string role) =>
        value is bool flag ? flag : throw Reject("data", role + "-not-boolean", value);

    internal static string RequireString(object? value, string role) =>
        value is string text ? text : throw Reject("data", role + "-not-string", value);

    private static bool IsSimpleSymbolName(string value)
    {
        if (value.Length == 0 || !IsSimpleSymbolInitial(value[0]))
        {
            return false;
        }

        for (int index = 1; index < value.Length; index++)
        {
            if (!IsSimpleSymbolInitial(value[index]) && !char.IsAsciiDigit(value[index]))
            {
                return false;
            }
        }

        return true;
    }

    private static bool IsSimpleSymbolInitial(char value) =>
        char.IsAsciiLetter(value) || value is '_' or '*' or '!' or '+' or '?' or '<' or '>' or '=' or '$' or '%' or '-';

    private static PersistentEnv RequireEnv(object? value) =>
        value as PersistentEnv ?? throw Reject("data", "not-environment", value);

    internal static CompilerRejectionException Reject(string phase, string code, object? evidence) =>
        new(phase, code, evidence);
}
