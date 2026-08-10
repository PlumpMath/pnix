using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.ExceptionServices;
using System.Runtime.Loader;
using System.Text;

namespace Pnix.ClrMeta.CompilerSupport;

public static class Program
{
    public static int Main(string[] args)
    {
        try
        {
            if (args.Length == 0)
            {
                throw DataAbi.Reject("host", "missing-command", null);
            }

            return args[0] switch
            {
                "compile" => Compile(args),
                "invoke-i64" => InvokeI64(args),
                "invoke-values" => InvokeValues(args),
                "describe" => Describe(args),
                "prepare" => Prepare(args),
                "contains-text" => ContainsText(args),
                "publish-directory" => PublishDirectory(args),
                _ => throw DataAbi.Reject("host", "unknown-command", args[0]),
            };
        }
        catch (Exception error)
        {
            Exception actual = Unwrap(error);
            Console.Error.WriteLine(ErrorJson(actual));
            return 2;
        }
    }

    private static int Compile(string[] args)
    {
        RequireArity(args, 4, "compile <compiler.dll> <source.clj> <output.dll>");
        string source = ReaderAbi.ReadStrictUtf8File(args[2]);
        var sink = new PeSink(args[3]);
        MethodInfo entry = LoadEntry(args[1]);
        object? result = InvokeEntry(entry, source, sink);
        if (result is not ArtifactDescriptor descriptor)
        {
            throw DataAbi.Reject("host", "compiler-result-not-artifact-descriptor", result);
        }

        Console.Out.WriteLine(DescriptorJson(descriptor));
        return 0;
    }

    private static int InvokeI64(string[] args)
    {
        RequireArity(args, 4, "invoke-i64 <artifact.dll> <left> <right>");
        long left = ParseI64(args[2]);
        long right = ParseI64(args[3]);
        MethodInfo entry = LoadEntry(args[1]);
        object? result = InvokeEntry(entry, left, right);
        Console.Out.WriteLine(ValueText(result));
        return 0;
    }

    private static int InvokeValues(string[] args)
    {
        RequireArity(args, 4, "invoke-values <artifact.dll> <left> <right>");
        MethodInfo entry = LoadEntry(args[1]);
        object? result = InvokeEntry(entry, ParseValue(args[2]), ParseValue(args[3]));
        Console.Out.WriteLine(ValueText(result));
        return 0;
    }

    private static int Describe(string[] args)
    {
        RequireArity(args, 2, "describe <artifact.dll>");
        Assembly assembly = LoadArtifact(args[1]);
        Type generated = RequireGeneratedType(assembly);
        var metadata = assembly
            .GetCustomAttributes<AssemblyMetadataAttribute>()
            .OrderBy(attribute => attribute.Key, StringComparer.Ordinal)
            .ToDictionary(attribute => attribute.Key, attribute => attribute.Value ?? string.Empty, StringComparer.Ordinal);
        string[] references = assembly.GetReferencedAssemblies()
            .Select(reference => reference.Name ?? string.Empty)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();
        string[] resources = assembly.GetManifestResourceNames().OrderBy(name => name, StringComparer.Ordinal).ToArray();
        string[] methods = generated.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(method => method.Name + "/" + method.GetParameters().Length.ToString(CultureInfo.InvariantCulture))
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();
        int fields = generated.GetFields(BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.DeclaredOnly).Length;

        Console.Out.WriteLine("{" +
            "\"assembly\":" + JsonString(assembly.GetName().Name ?? string.Empty) + "," +
            "\"fields\":" + fields.ToString(CultureInfo.InvariantCulture) + "," +
            "\"metadata\":" + JsonObject(metadata) + "," +
            "\"methods\":" + JsonArray(methods) + "," +
            "\"references\":" + JsonArray(references) + "," +
            "\"resources\":" + JsonArray(resources) + "," +
            "\"schema\":\"pnix.clr-meta.compiler-selfhost-description.v1\"," +
            "\"type\":" + JsonString(generated.FullName ?? string.Empty) +
            "}");
        return 0;
    }

    private static int Prepare(string[] args)
    {
        RequireArity(args, 2, "prepare <artifact.dll>");
        Assembly assembly = LoadArtifact(args[1]);
        Type generated = RequireGeneratedType(assembly);
        MethodInfo[] methods = generated.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        foreach (MethodInfo method in methods)
        {
            RuntimeHelpers.PrepareMethod(method.MethodHandle);
        }

        Console.Out.WriteLine(methods.Length.ToString(CultureInfo.InvariantCulture));
        return 0;
    }

    private static int ContainsText(string[] args)
    {
        RequireArity(args, 3, "contains-text <artifact.dll> <text>");
        string full = RequireRegularFile(args[1], "artifact-missing", "artifact-reparse-point");
        if (args[2].Length == 0)
        {
            throw DataAbi.Reject("host", "empty-search-text", null);
        }

        var info = new FileInfo(full);
        if (info.Length > 16 * 1024 * 1024)
        {
            throw DataAbi.Reject("host", "artifact-inspection-byte-budget", info.Length);
        }

        byte[] bytes = File.ReadAllBytes(full);
        bool found = ContainsSequence(bytes, Encoding.UTF8.GetBytes(args[2])) ||
            ContainsSequence(bytes, Encoding.Unicode.GetBytes(args[2]));
        Console.Out.WriteLine(found ? "true" : "false");
        return 0;
    }

    private static int PublishDirectory(string[] args)
    {
        RequireArity(args, 3, "publish-directory <staging-directory> <output-directory>");
        string source = Path.GetFullPath(args[1]);
        string output = Path.GetFullPath(args[2]);
        if (!Directory.Exists(source))
        {
            throw DataAbi.Reject("host", "staging-directory-missing", source);
        }

        if ((File.GetAttributes(source) & FileAttributes.ReparsePoint) != 0)
        {
            throw DataAbi.Reject("host", "staging-directory-reparse-point", source);
        }

        string? parent = Path.GetDirectoryName(output);
        if (parent is null || !Directory.Exists(parent))
        {
            throw DataAbi.Reject("host", "output-parent-missing", parent);
        }

        if (PathEntryExists(output))
        {
            throw DataAbi.Reject("host", "output-exists", output);
        }

        Directory.Move(source, output);
        return 0;
    }

    private static object? InvokeEntry(MethodInfo entry, object? left, object? right)
    {
        try
        {
            return entry.Invoke(null, new[] { left, right });
        }
        catch (TargetInvocationException error) when (error.InnerException is not null)
        {
            ExceptionDispatchInfo.Capture(error.InnerException).Throw();
            throw;
        }
    }

    private static MethodInfo LoadEntry(string artifactPath)
    {
        Assembly assembly = LoadArtifact(artifactPath);
        if (!string.Equals(assembly.GetName().Name, PeSink.GeneratedAssemblyName, StringComparison.Ordinal))
        {
            throw DataAbi.Reject("host", "generated-assembly-name", assembly.GetName().Name);
        }

        Type generated = RequireGeneratedType(assembly);
        var metadata = assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .ToDictionary(attribute => attribute.Key, attribute => attribute.Value ?? string.Empty, StringComparer.Ordinal);
        string[] requiredKeys =
        {
            "GeneratedBy", "KernelEntry", "KernelIdentity", "KernelNamespace", "SourceProfileId", "SupportAbiId",
        };
        if (metadata.Count != requiredKeys.Length || requiredKeys.Any(key => !metadata.ContainsKey(key)))
        {
            throw DataAbi.Reject("host", "artifact-metadata-closure", metadata.Keys.OrderBy(key => key).ToArray());
        }

        RequireMetadata(metadata, "GeneratedBy", "pnix.clr-meta.compiler-kernel.v1");
        RequireMetadata(metadata, "KernelEntry", "compile-source");
        RequireMetadata(metadata, "KernelNamespace", "pnix.clr-meta.compiler-kernel.v1");
        RequireMetadata(metadata, "SourceProfileId", "pnix.clr-meta.compiler-kernel-source.v1");
        RequireMetadata(metadata, "SupportAbiId", PeSink.SupportAbiId);
        if (metadata["KernelIdentity"].Length == 0)
        {
            throw DataAbi.Reject("host", "empty-kernel-identity", null);
        }

        string entryName = metadata["KernelEntry"];
        MethodInfo? entry = generated
            .GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .SingleOrDefault(method => string.Equals(method.Name, entryName, StringComparison.Ordinal));
        if (entry is null || entry.ReturnType != typeof(object))
        {
            throw DataAbi.Reject("host", "entry-method-missing", entryName);
        }

        ParameterInfo[] parameters = entry.GetParameters();
        if (parameters.Length != 2 || parameters.Any(parameter => parameter.ParameterType != typeof(object)))
        {
            throw DataAbi.Reject("host", "entry-method-signature", entryName);
        }

        return entry;
    }

    private static Assembly LoadArtifact(string path)
    {
        string full = RequireRegularFile(path, "artifact-missing", "artifact-reparse-point");
        return AssemblyLoadContext.Default.LoadFromAssemblyPath(full);
    }

    private static Type RequireGeneratedType(Assembly assembly) =>
        assembly.GetType(PeSink.GeneratedTypeName, false, false)
        ?? throw DataAbi.Reject("host", "generated-type-missing", PeSink.GeneratedTypeName);

    private static long ParseI64(string text)
    {
        if (!long.TryParse(text, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out long value))
        {
            throw DataAbi.Reject("host", "invalid-i64", text);
        }

        return value;
    }

    private static object? ParseValue(string text)
    {
        if (string.Equals(text, "nil", StringComparison.Ordinal))
        {
            return null;
        }

        if (string.Equals(text, "true", StringComparison.Ordinal))
        {
            return true;
        }

        if (string.Equals(text, "false", StringComparison.Ordinal))
        {
            return false;
        }

        if (text.StartsWith("i64:", StringComparison.Ordinal))
        {
            return ParseI64(text.Substring(4));
        }

        if (text.StartsWith("string:", StringComparison.Ordinal))
        {
            return text.Substring(7);
        }

        throw DataAbi.Reject("host", "invalid-value-token", text);
    }

    private static string ValueText(object? value) => value switch
    {
        null => "nil",
        bool flag => flag ? "true" : "false",
        long number => number.ToString(CultureInfo.InvariantCulture),
        string text => text,
        _ => throw DataAbi.Reject("host", "unsupported-result-value", value),
    };

    private static string RequireRegularFile(string path, string missingCode, string reparseCode)
    {
        string full = Path.GetFullPath(path);
        if (!File.Exists(full))
        {
            throw DataAbi.Reject("host", missingCode, full);
        }

        if ((File.GetAttributes(full) & FileAttributes.ReparsePoint) != 0)
        {
            throw DataAbi.Reject("host", reparseCode, full);
        }

        return full;
    }

    private static bool PathEntryExists(string path)
    {
        try
        {
            _ = File.GetAttributes(path);
            return true;
        }
        catch (FileNotFoundException)
        {
            return false;
        }
        catch (DirectoryNotFoundException)
        {
            return false;
        }
    }

    private static void RequireMetadata(IReadOnlyDictionary<string, string> metadata, string key, string expected)
    {
        if (!metadata.TryGetValue(key, out string? actual) || !string.Equals(actual, expected, StringComparison.Ordinal))
        {
            throw DataAbi.Reject("host", "artifact-metadata-" + key, actual);
        }
    }

    private static bool ContainsSequence(byte[] haystack, byte[] needle)
    {
        if (needle.Length == 0 || needle.Length > haystack.Length)
        {
            return false;
        }

        for (int start = 0; start <= haystack.Length - needle.Length; start++)
        {
            int index = 0;
            while (index < needle.Length && haystack[start + index] == needle[index])
            {
                index++;
            }

            if (index == needle.Length)
            {
                return true;
            }
        }

        return false;
    }

    private static string DescriptorJson(ArtifactDescriptor descriptor) =>
        "{" +
        "\"kernel_entry\":" + JsonString(descriptor.KernelEntry) + "," +
        "\"kernel_identity\":" + JsonString(descriptor.KernelIdentity) + "," +
        "\"kernel_namespace\":" + JsonString(descriptor.KernelNamespace) + "," +
        "\"path\":" + JsonString(descriptor.Path) + "," +
        "\"schema\":\"pnix.clr-meta.compiler-selfhost-artifact-descriptor.v1\"," +
        "\"sha256\":" + JsonString(descriptor.Sha256) + "," +
        "\"source_profile_id\":" + JsonString(descriptor.SourceProfileId) +
        "}";

    private static string ErrorJson(Exception error)
    {
        if (error is CompilerRejectionException rejection)
        {
            return "{" +
                "\"class\":" + JsonString(rejection.Code) + "," +
                "\"message\":" + JsonString(rejection.Message) + "," +
                "\"phase\":" + JsonString(rejection.Phase) + "," +
                "\"schema\":\"pnix.clr-meta.compiler-selfhost-error.v1\"" +
                "}";
        }

        return "{" +
            "\"class\":\"infrastructure-failure\"," +
            "\"message\":" + JsonString(error.Message) + "," +
            "\"phase\":\"host\"," +
            "\"schema\":\"pnix.clr-meta.compiler-selfhost-error.v1\"," +
            "\"type\":" + JsonString(error.GetType().FullName ?? error.GetType().Name) +
            "}";
    }

    private static Exception Unwrap(Exception error)
    {
        while (error is TargetInvocationException { InnerException: not null } target)
        {
            error = target.InnerException!;
        }

        return error;
    }

    private static void RequireArity(string[] args, int expected, string usage)
    {
        if (args.Length != expected)
        {
            throw DataAbi.Reject("host", "command-arity", usage);
        }
    }

    private static string JsonObject(IReadOnlyDictionary<string, string> value) =>
        "{" + string.Join(",", value.Select(pair => JsonString(pair.Key) + ":" + JsonString(pair.Value))) + "}";

    private static string JsonArray(IEnumerable<string> value) =>
        "[" + string.Join(",", value.Select(JsonString)) + "]";

    private static string JsonString(string value)
    {
        var builder = new StringBuilder(value.Length + 2);
        builder.Append('"');
        foreach (char current in value)
        {
            switch (current)
            {
                case '"': builder.Append("\\\""); break;
                case '\\': builder.Append("\\\\"); break;
                case '\b': builder.Append("\\b"); break;
                case '\f': builder.Append("\\f"); break;
                case '\n': builder.Append("\\n"); break;
                case '\r': builder.Append("\\r"); break;
                case '\t': builder.Append("\\t"); break;
                default:
                    if (current < ' ')
                    {
                        builder.Append("\\u");
                        builder.Append(((int)current).ToString("x4", CultureInfo.InvariantCulture));
                    }
                    else
                    {
                        builder.Append(current);
                    }
                    break;
            }
        }

        builder.Append('"');
        return builder.ToString();
    }
}
