using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;
using System.Security.Cryptography;

namespace Pnix.ClrMeta.CompilerSupport;

public sealed class ArtifactDescriptor
{
    internal ArtifactDescriptor(
        string path,
        string kernelIdentity,
        string kernelNamespace,
        string kernelEntry,
        string sourceProfileId,
        string sha256)
    {
        Path = path;
        KernelIdentity = kernelIdentity;
        KernelNamespace = kernelNamespace;
        KernelEntry = kernelEntry;
        SourceProfileId = sourceProfileId;
        Sha256 = sha256;
    }

    public string Path { get; }
    public string KernelIdentity { get; }
    public string KernelNamespace { get; }
    public string KernelEntry { get; }
    public string SourceProfileId { get; }
    public string Sha256 { get; }
}

internal sealed class LocalHandle
{
    internal LocalHandle(Guid owner, LocalBuilder local)
    {
        Owner = owner;
        Local = local;
    }

    internal Guid Owner { get; }
    internal LocalBuilder Local { get; }
}

internal sealed class LabelHandle
{
    internal LabelHandle(Guid owner, Label label)
    {
        Owner = owner;
        Label = label;
    }

    internal Guid Owner { get; }
    internal Label Label { get; }
    internal int? ExpectedStackHeight { get; set; }
    internal bool Marked { get; set; }
}

internal sealed class MethodRecord
{
    internal MethodRecord(MethodBuilder builder, long arity)
    {
        Builder = builder;
        Arity = arity;
    }

    internal MethodBuilder Builder { get; }
    internal long Arity { get; }
    internal bool Completed { get; set; }
}

public static class RuntimeOps
{
    public static object ClosedEquals(object? left, object? right)
    {
        if (left is null || right is null)
        {
            return left is null && right is null;
        }

        return left switch
        {
            bool leftBool when right is bool rightBool => leftBool == rightBool,
            long leftLong when right is long rightLong => leftLong == rightLong,
            string leftString when right is string rightString =>
                string.Equals(leftString, rightString, StringComparison.Ordinal),
            _ => false,
        };
    }

    public static bool IsTruthy(object? value) => value is not null && value is not false;
}

public sealed class PeSink
{
    public const string SupportAbiId = "pnix.clr-meta.compiler-support.v1";
    public const string GeneratedAssemblyName = "Pnix.ClrMeta.Generated.CompilerKernel";
    public const string GeneratedTypeName = "Pnix.ClrMeta.Generated.CompilerKernel";

    private readonly string outputPath;
    private readonly Dictionary<string, FieldBuilder> fields = new(StringComparer.Ordinal);
    private readonly Dictionary<string, MethodRecord> methods = new(StringComparer.Ordinal);
    private readonly List<LabelHandle> currentLabels = new();

    private PersistedAssemblyBuilder? assembly;
    private TypeBuilder? generatedType;
    private ILGenerator? currentIl;
    private MethodRecord? currentMethod;
    private bool initializerOpen;
    private bool initializerComplete;
    private bool begun;
    private bool finished;
    private bool currentReturned;
    private bool currentReachable;
    private Guid currentBodyOwner;
    private int currentStackHeight;
    private string? kernelIdentity;
    private string? kernelNamespace;
    private string? kernelEntry;
    private string? sourceProfileId;

    public PeSink(string outputPath)
    {
        this.outputPath = System.IO.Path.GetFullPath(outputPath);
        string? parent = System.IO.Path.GetDirectoryName(this.outputPath);
        if (parent is null || !Directory.Exists(parent))
        {
            throw DataAbi.Reject("pesink", "output-parent-missing", parent);
        }

        if (File.Exists(this.outputPath) || Directory.Exists(this.outputPath))
        {
            throw DataAbi.Reject("pesink", "output-exists", this.outputPath);
        }
    }

    internal object? Begin(object? identityValue, object? namespaceValue, object? entryValue, object? profileValue)
    {
        RequireNotFinished();
        if (begun)
        {
            throw DataAbi.Reject("pesink", "begin-twice", null);
        }

        kernelIdentity = DataAbi.RequireString(identityValue, "kernel-identity");
        kernelNamespace = DataAbi.RequireString(namespaceValue, "kernel-namespace");
        kernelEntry = DataAbi.RequireString(entryValue, "kernel-entry");
        sourceProfileId = DataAbi.RequireString(profileValue, "source-profile-id");
        if (kernelIdentity.Length == 0 || kernelNamespace.Length == 0 || kernelEntry.Length == 0 || sourceProfileId.Length == 0)
        {
            throw DataAbi.Reject("pesink", "empty-begin-identity", null);
        }

        var assemblyName = new AssemblyName(GeneratedAssemblyName)
        {
            Version = new Version(1, 0, 0, 0),
        };
        assembly = new PersistedAssemblyBuilder(assemblyName, typeof(object).Assembly);
        ModuleBuilder module = assembly.DefineDynamicModule(GeneratedAssemblyName);
        generatedType = module.DefineType(
            GeneratedTypeName,
            TypeAttributes.Public | TypeAttributes.Abstract | TypeAttributes.Sealed,
            typeof(object));
        begun = true;
        return null;
    }

    internal object? DefineConstant(object? nameValue)
    {
        TypeBuilder type = RequireDefinitionPhase();
        string name = DataAbi.RequireString(nameValue, "constant-name");
        if (fields.ContainsKey(name) || methods.ContainsKey(name))
        {
            throw DataAbi.Reject("pesink", "duplicate-definition", name);
        }

        fields.Add(name, type.DefineField(name, typeof(object), FieldAttributes.Private | FieldAttributes.Static));
        return null;
    }

    internal object? DefineMethod(object? nameValue, object? arityValue)
    {
        TypeBuilder type = RequireDefinitionPhase();
        string name = DataAbi.RequireString(nameValue, "method-name");
        long arity = DataAbi.RequireInt64(arityValue, "method-arity");
        if (arity < 0 || arity > 5)
        {
            throw DataAbi.Reject("pesink", "method-arity-range", arity);
        }

        if (fields.ContainsKey(name) || methods.ContainsKey(name))
        {
            throw DataAbi.Reject("pesink", "duplicate-definition", name);
        }

        var parameterTypes = new Type[arity];
        Array.Fill(parameterTypes, typeof(object));
        MethodBuilder builder = type.DefineMethod(
            name,
            MethodAttributes.Public | MethodAttributes.Static,
            typeof(object),
            parameterTypes);
        methods.Add(name, new MethodRecord(builder, arity));
        return null;
    }

    internal object? BeginInitializer()
    {
        TypeBuilder type = RequireDefinitionPhase();
        if (initializerOpen || initializerComplete)
        {
            throw DataAbi.Reject("pesink", "initializer-state", null);
        }

        currentIl = type.DefineTypeInitializer().GetILGenerator();
        initializerOpen = true;
        ResetBodyState();
        return null;
    }

    internal object? EndInitializer()
    {
        if (!initializerOpen || currentIl is null || currentMethod is not null)
        {
            throw DataAbi.Reject("pesink", "initializer-not-open", null);
        }

        if (!currentReachable || currentStackHeight != 0)
        {
            throw DataAbi.Reject("pesink", "initializer-stack-state", currentStackHeight);
        }

        RequireAllLabelsMarked();

        currentIl.Emit(OpCodes.Ret);
        currentIl = null;
        initializerOpen = false;
        initializerComplete = true;
        currentReturned = false;
        currentReachable = false;
        currentLabels.Clear();
        return null;
    }

    internal object? BeginMethod(object? nameValue, object? arityValue)
    {
        RequireBegun();
        if (!initializerComplete || currentIl is not null)
        {
            throw DataAbi.Reject("pesink", "method-begin-state", null);
        }

        string name = DataAbi.RequireString(nameValue, "method-name");
        long arity = DataAbi.RequireInt64(arityValue, "method-arity");
        if (!methods.TryGetValue(name, out MethodRecord? method) || method.Arity != arity || method.Completed)
        {
            throw DataAbi.Reject("pesink", "undefined-or-complete-method", name);
        }

        currentMethod = method;
        currentIl = method.Builder.GetILGenerator();
        ResetBodyState();
        return null;
    }

    internal object? EndMethod()
    {
        if (currentMethod is null || currentIl is null || initializerOpen || !currentReturned)
        {
            throw DataAbi.Reject("pesink", "method-end-state", null);
        }

        if (currentStackHeight != 0 || currentReachable)
        {
            throw DataAbi.Reject("pesink", "method-stack-state", currentStackHeight);
        }

        RequireAllLabelsMarked();

        currentMethod.Completed = true;
        currentMethod = null;
        currentIl = null;
        currentReturned = false;
        currentReachable = false;
        currentLabels.Clear();
        return null;
    }

    internal object AllocateLocal()
    {
        ILGenerator il = RequireMethodIl();
        return new LocalHandle(currentBodyOwner, il.DeclareLocal(typeof(object)));
    }

    internal object NewLabel()
    {
        ILGenerator il = RequireMethodIl();
        var handle = new LabelHandle(currentBodyOwner, il.DefineLabel());
        currentLabels.Add(handle);
        return handle;
    }

    internal object? MarkLabel(object? handleValue)
    {
        ILGenerator il = RequireOpenMethodIl();
        LabelHandle handle = RequireLabel(handleValue);
        if (handle.Marked)
        {
            throw DataAbi.Reject("pesink", "label-marked-twice", null);
        }

        if (currentReturned)
        {
            throw DataAbi.Reject("pesink", "emission-after-ret", null);
        }

        if (currentReachable)
        {
            RequireLabelStack(handle, currentStackHeight);
        }
        else if (handle.ExpectedStackHeight is int expected)
        {
            currentStackHeight = expected;
            currentReachable = true;
        }
        else
        {
            throw DataAbi.Reject("pesink", "unreachable-unreferenced-label", null);
        }

        il.MarkLabel(handle.Label);
        handle.Marked = true;
        return null;
    }

    internal object? EmitLiteral(object? kindValue, object? value)
    {
        ILGenerator il = RequireBodyIl();
        string kind = DataAbi.RequireString(kindValue, "literal-kind");
        switch (kind)
        {
            case "nil" when value is null:
                il.Emit(OpCodes.Ldnull);
                break;
            case "boolean" when value is bool flag:
                il.Emit(flag ? OpCodes.Ldc_I4_1 : OpCodes.Ldc_I4_0);
                il.Emit(OpCodes.Box, typeof(bool));
                break;
            case "int64" when value is long number:
                il.Emit(OpCodes.Ldc_I8, number);
                il.Emit(OpCodes.Box, typeof(long));
                break;
            case "string" when value is string text:
                il.Emit(OpCodes.Ldstr, text);
                break;
            default:
                throw DataAbi.Reject("pesink", "literal-kind-value-mismatch", kind);
        }

        PushStack();
        return null;
    }

    internal object? EmitLoadArg(object? indexValue)
    {
        ILGenerator il = RequireMethodIl();
        long index = DataAbi.RequireInt64(indexValue, "argument-index");
        if (currentMethod is null || index < 0 || index >= currentMethod.Arity)
        {
            throw DataAbi.Reject("pesink", "argument-index-range", index);
        }

        il.Emit(OpCodes.Ldarg, checked((short)index));
        PushStack();
        return null;
    }

    internal object? EmitLoadLocal(object? handleValue)
    {
        RequireBodyIl().Emit(OpCodes.Ldloc, RequireLocal(handleValue).Local);
        PushStack();
        return null;
    }

    internal object? EmitLoadField(object? nameValue)
    {
        string name = DataAbi.RequireString(nameValue, "field-name");
        if (!fields.TryGetValue(name, out FieldBuilder? field))
        {
            throw DataAbi.Reject("pesink", "undefined-field", name);
        }

        RequireBodyIl().Emit(OpCodes.Ldsfld, field);
        PushStack();
        return null;
    }

    internal object? EmitStoreLocal(object? handleValue)
    {
        ILGenerator il = RequireBodyIl();
        LocalHandle handle = RequireLocal(handleValue);
        PopStack(1, "store-local");
        il.Emit(OpCodes.Stloc, handle.Local);
        return null;
    }

    internal object? EmitStoreField(object? nameValue)
    {
        if (!initializerOpen)
        {
            throw DataAbi.Reject("pesink", "field-store-outside-initializer", null);
        }

        string name = DataAbi.RequireString(nameValue, "field-name");
        if (!fields.TryGetValue(name, out FieldBuilder? field))
        {
            throw DataAbi.Reject("pesink", "undefined-field", name);
        }

        ILGenerator il = RequireBodyIl();
        PopStack(1, "store-field");
        il.Emit(OpCodes.Stsfld, field);
        return null;
    }

    internal object? EmitCall(object? targetValue, object? arityValue)
    {
        ILGenerator il = RequireBodyIl();
        string target = DataAbi.RequireString(targetValue, "call-target");
        long arity = DataAbi.RequireInt64(arityValue, "call-arity");
        if (methods.TryGetValue(target, out MethodRecord? method))
        {
            if (method.Arity != arity)
            {
                throw DataAbi.Reject("pesink", "kernel-call-arity", target);
            }

            PopStack(checked((int)arity), "kernel-call");
            il.Emit(OpCodes.Call, method.Builder);
            PushStack();
            return null;
        }

        if (!SupportMethods.All.TryGetValue(target, out MethodInfo? support) || support.GetParameters().LongLength != arity)
        {
            throw DataAbi.Reject("pesink", "unknown-support-call", target);
        }

        PopStack(checked((int)arity), "support-call");
        il.Emit(OpCodes.Call, support);
        PushStack();
        return null;
    }

    internal object? EmitOpcode(object? opcodeValue)
    {
        ILGenerator il = RequireBodyIl();
        string opcode = DataAbi.RequireString(opcodeValue, "opcode");
        if (opcode is not ("add.ovf" or "sub.ovf" or "ceq" or "clt"))
        {
            throw DataAbi.Reject("pesink", "unknown-opcode", opcode);
        }

        PopStack(2, "opcode");
        switch (opcode)
        {
            case "add.ovf":
                EmitCheckedBinary(il, OpCodes.Add_Ovf);
                break;
            case "sub.ovf":
                EmitCheckedBinary(il, OpCodes.Sub_Ovf);
                break;
            case "ceq":
                il.Emit(OpCodes.Call, SupportMethods.ClosedEquals);
                break;
            case "clt":
                EmitComparison(il, OpCodes.Clt);
                break;
        }

        PushStack();
        return null;
    }

    internal object? EmitBranchFalse(object? handleValue)
    {
        ILGenerator il = RequireMethodIl();
        LabelHandle handle = RequireLabel(handleValue);
        PopStack(1, "branch-false");
        RequireLabelStack(handle, currentStackHeight);
        il.Emit(OpCodes.Call, SupportMethods.IsTruthy);
        il.Emit(OpCodes.Brfalse, handle.Label);
        return null;
    }

    internal object? EmitBranch(object? handleValue)
    {
        ILGenerator il = RequireMethodIl();
        LabelHandle handle = RequireLabel(handleValue);
        RequireLabelStack(handle, currentStackHeight);
        il.Emit(OpCodes.Br, handle.Label);
        currentReachable = false;
        return null;
    }

    internal object? EmitPop()
    {
        ILGenerator il = RequireBodyIl();
        PopStack(1, "pop");
        il.Emit(OpCodes.Pop);
        return null;
    }

    internal object? EmitRet()
    {
        if (initializerOpen)
        {
            throw DataAbi.Reject("pesink", "explicit-initializer-ret", null);
        }

        ILGenerator il = RequireMethodIl();
        if (currentStackHeight != 1)
        {
            throw DataAbi.Reject("pesink", "return-stack-height", currentStackHeight);
        }

        il.Emit(OpCodes.Ret);
        currentStackHeight = 0;
        currentReachable = false;
        currentReturned = true;
        return null;
    }

    internal object Finish()
    {
        RequireBegun();
        if (finished || currentIl is not null || !initializerComplete)
        {
            throw DataAbi.Reject("pesink", "finish-state", null);
        }

        foreach ((string name, MethodRecord method) in methods)
        {
            if (!method.Completed)
            {
                throw DataAbi.Reject("pesink", "method-not-complete", name);
            }
        }

        if (!methods.ContainsKey(kernelEntry!))
        {
            throw DataAbi.Reject("pesink", "entry-method-missing", kernelEntry);
        }

        AddAssemblyMetadata("KernelIdentity", kernelIdentity!);
        AddAssemblyMetadata("KernelNamespace", kernelNamespace!);
        AddAssemblyMetadata("KernelEntry", kernelEntry!);
        AddAssemblyMetadata("SourceProfileId", sourceProfileId!);
        AddAssemblyMetadata("SupportAbiId", SupportAbiId);
        AddAssemblyMetadata("GeneratedBy", "pnix.clr-meta.compiler-kernel.v1");
        generatedType!.CreateType();

        string temporary = outputPath + ".tmp." + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture);
        try
        {
            if (File.Exists(outputPath) || Directory.Exists(outputPath))
            {
                throw DataAbi.Reject("pesink", "output-exists-at-finish", outputPath);
            }

            assembly!.Save(temporary);
            CanonicalizeForReproducibility(temporary);
            File.Move(temporary, outputPath);
        }
        finally
        {
            if (File.Exists(temporary))
            {
                File.Delete(temporary);
            }
        }

        finished = true;
        return new ArtifactDescriptor(
            outputPath,
            kernelIdentity!,
            kernelNamespace!,
            kernelEntry!,
            sourceProfileId!,
            Sha256File(outputPath));
    }

    private static void EmitCheckedBinary(ILGenerator il, OpCode opcode)
    {
        LocalBuilder right = il.DeclareLocal(typeof(long));
        LocalBuilder left = il.DeclareLocal(typeof(long));
        il.Emit(OpCodes.Unbox_Any, typeof(long));
        il.Emit(OpCodes.Stloc, right);
        il.Emit(OpCodes.Unbox_Any, typeof(long));
        il.Emit(OpCodes.Stloc, left);
        il.Emit(OpCodes.Ldloc, left);
        il.Emit(OpCodes.Ldloc, right);
        il.Emit(opcode);
        il.Emit(OpCodes.Box, typeof(long));
    }

    private static void EmitComparison(ILGenerator il, OpCode opcode)
    {
        LocalBuilder right = il.DeclareLocal(typeof(long));
        LocalBuilder left = il.DeclareLocal(typeof(long));
        il.Emit(OpCodes.Unbox_Any, typeof(long));
        il.Emit(OpCodes.Stloc, right);
        il.Emit(OpCodes.Unbox_Any, typeof(long));
        il.Emit(OpCodes.Stloc, left);
        il.Emit(OpCodes.Ldloc, left);
        il.Emit(OpCodes.Ldloc, right);
        il.Emit(opcode);
        il.Emit(OpCodes.Box, typeof(bool));
    }

    private void AddAssemblyMetadata(string key, string value)
    {
        ConstructorInfo constructor = typeof(AssemblyMetadataAttribute).GetConstructor(new[] { typeof(string), typeof(string) })!;
        assembly!.SetCustomAttribute(new CustomAttributeBuilder(constructor, new object[] { key, value }));
    }

    private TypeBuilder RequireDefinitionPhase()
    {
        RequireBegun();
        if (initializerOpen || initializerComplete || currentIl is not null)
        {
            throw DataAbi.Reject("pesink", "definition-phase-closed", null);
        }

        return generatedType!;
    }

    private ILGenerator RequireBodyIl()
    {
        RequireBegun();
        ILGenerator il = currentIl ?? throw DataAbi.Reject("pesink", "no-open-body", null);
        if (currentReturned)
        {
            throw DataAbi.Reject("pesink", "emission-after-ret", null);
        }

        if (!currentReachable)
        {
            throw DataAbi.Reject("pesink", "emission-in-unreachable-body", null);
        }

        return il;
    }

    private ILGenerator RequireMethodIl()
    {
        RequireOpenMethodIl();
        return RequireBodyIl();
    }

    private ILGenerator RequireOpenMethodIl()
    {
        if (currentMethod is null || initializerOpen)
        {
            throw DataAbi.Reject("pesink", "no-open-method", null);
        }

        return currentIl ?? throw DataAbi.Reject("pesink", "no-open-body", null);
    }

    private LocalHandle RequireLocal(object? value)
    {
        if (value is not LocalHandle handle || handle.Owner != currentBodyOwner)
        {
            throw DataAbi.Reject("pesink", "foreign-local-handle", value);
        }

        return handle;
    }

    private LabelHandle RequireLabel(object? value)
    {
        if (value is not LabelHandle handle || handle.Owner != currentBodyOwner)
        {
            throw DataAbi.Reject("pesink", "foreign-label-handle", value);
        }

        return handle;
    }

    private void ResetBodyState()
    {
        currentBodyOwner = Guid.NewGuid();
        currentStackHeight = 0;
        currentReachable = true;
        currentReturned = false;
        currentLabels.Clear();
    }

    private void PushStack() => currentStackHeight++;

    private void PopStack(int count, string operation)
    {
        if (count < 0 || currentStackHeight < count)
        {
            throw DataAbi.Reject("pesink", "stack-underflow-" + operation, currentStackHeight);
        }

        currentStackHeight -= count;
    }

    private static void RequireLabelStack(LabelHandle handle, int stackHeight)
    {
        if (handle.ExpectedStackHeight is int expected && expected != stackHeight)
        {
            throw DataAbi.Reject("pesink", "label-stack-height-mismatch", stackHeight);
        }

        handle.ExpectedStackHeight ??= stackHeight;
    }

    private void RequireAllLabelsMarked()
    {
        foreach (LabelHandle label in currentLabels)
        {
            if (!label.Marked)
            {
                throw DataAbi.Reject("pesink", "unmarked-label", null);
            }
        }
    }

    private void RequireBegun()
    {
        RequireNotFinished();
        if (!begun)
        {
            throw DataAbi.Reject("pesink", "not-begun", null);
        }
    }

    private void RequireNotFinished()
    {
        if (finished)
        {
            throw DataAbi.Reject("pesink", "already-finished", null);
        }
    }

    private static string Sha256File(string path)
    {
        using FileStream stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

    // Stage8: PersistedAssemblyBuilder.Save() has no public hook to control the
    // PE COFF header TimeDateStamp or the module's MVID -- both vary between
    // two saves of byte-identical IL, confirmed empirically (two builds from
    // the same frozen source differed at exactly these two locations and
    // nowhere else). Neither field is read by anything this pipeline invokes
    // (compile/invoke/describe/prepare never inspect them), so zeroing both
    // after Save() is a safe, purely-cosmetic patch that makes the artifact
    // byte-reproducible across independent builds. The MVID patch locates its
    // exact bytes via the real GUID that MetadataReader reports (not a
    // fixed offset, since heap layout shifts with content) and requires that
    // 16-byte sequence to occur exactly once in the file before overwriting it
    // -- a 128-bit random value colliding elsewhere in a ~20KB IL image is not
    // a real risk, but failing closed on anything else is cheap insurance.
    private static void CanonicalizeForReproducibility(string path)
    {
        byte[] bytes = File.ReadAllBytes(path);
        int timeDateStampOffset;
        byte[] mvidBytes;
        using (var stream = new MemoryStream(bytes, writable: false))
        using (var peReader = new PEReader(stream))
        {
            timeDateStampOffset = peReader.PEHeaders.CoffHeaderStartOffset + 4;
            MetadataReader metadataReader = peReader.GetMetadataReader();
            ModuleDefinition module = metadataReader.GetModuleDefinition();
            mvidBytes = metadataReader.GetGuid(module.Mvid).ToByteArray();
        }

        for (int i = 0; i < 4; i++)
        {
            bytes[timeDateStampOffset + i] = 0;
        }

        int mvidOffset = FindSingleOccurrence(bytes, mvidBytes);
        for (int i = 0; i < mvidBytes.Length; i++)
        {
            bytes[mvidOffset + i] = 0;
        }

        File.WriteAllBytes(path, bytes);
    }

    private static int FindSingleOccurrence(byte[] haystack, byte[] needle)
    {
        int found = -1;
        int count = 0;
        for (int i = 0; i + needle.Length <= haystack.Length; i++)
        {
            bool match = true;
            for (int j = 0; j < needle.Length; j++)
            {
                if (haystack[i + j] != needle[j])
                {
                    match = false;
                    break;
                }
            }

            if (match)
            {
                count++;
                found = i;
            }
        }

        if (count != 1)
        {
            throw DataAbi.Reject("pesink", "mvid-occurrence-count", count);
        }

        return found;
    }
}

public static class PeSinkAbi
{
    public static object? Begin(object? sink, object? identity, object? kernelNamespace, object? entry, object? profile) =>
        RequireSink(sink).Begin(identity, kernelNamespace, entry, profile);

    public static object? DefineConstant(object? sink, object? name) => RequireSink(sink).DefineConstant(name);
    public static object? DefineMethod(object? sink, object? name, object? arity) => RequireSink(sink).DefineMethod(name, arity);
    public static object? BeginInitializer(object? sink) => RequireSink(sink).BeginInitializer();
    public static object? EndInitializer(object? sink) => RequireSink(sink).EndInitializer();
    public static object? BeginMethod(object? sink, object? name, object? arity) => RequireSink(sink).BeginMethod(name, arity);
    public static object? EndMethod(object? sink) => RequireSink(sink).EndMethod();
    public static object AllocateLocal(object? sink) => RequireSink(sink).AllocateLocal();
    public static object NewLabel(object? sink) => RequireSink(sink).NewLabel();
    public static object? MarkLabel(object? sink, object? label) => RequireSink(sink).MarkLabel(label);
    public static object? EmitLiteral(object? sink, object? kind, object? value) => RequireSink(sink).EmitLiteral(kind, value);
    public static object? EmitLoadArg(object? sink, object? index) => RequireSink(sink).EmitLoadArg(index);
    public static object? EmitLoadLocal(object? sink, object? local) => RequireSink(sink).EmitLoadLocal(local);
    public static object? EmitLoadField(object? sink, object? name) => RequireSink(sink).EmitLoadField(name);
    public static object? EmitStoreLocal(object? sink, object? local) => RequireSink(sink).EmitStoreLocal(local);
    public static object? EmitStoreField(object? sink, object? name) => RequireSink(sink).EmitStoreField(name);
    public static object? EmitCall(object? sink, object? target, object? arity) => RequireSink(sink).EmitCall(target, arity);
    public static object? EmitOpcode(object? sink, object? opcode) => RequireSink(sink).EmitOpcode(opcode);
    public static object? EmitBranchFalse(object? sink, object? label) => RequireSink(sink).EmitBranchFalse(label);
    public static object? EmitBranch(object? sink, object? label) => RequireSink(sink).EmitBranch(label);
    public static object? EmitPop(object? sink) => RequireSink(sink).EmitPop();
    public static object? EmitRet(object? sink) => RequireSink(sink).EmitRet();
    public static object Finish(object? sink) => RequireSink(sink).Finish();

    private static PeSink RequireSink(object? value) =>
        value as PeSink ?? throw DataAbi.Reject("pesink", "not-pesink", value);
}

internal static class SupportMethods
{
    internal static readonly MethodInfo ClosedEquals =
        typeof(RuntimeOps).GetMethod(nameof(RuntimeOps.ClosedEquals), BindingFlags.Public | BindingFlags.Static)!;
    internal static readonly MethodInfo IsTruthy =
        typeof(RuntimeOps).GetMethod(nameof(RuntimeOps.IsTruthy), BindingFlags.Public | BindingFlags.Static)!;

    internal static readonly IReadOnlyDictionary<string, MethodInfo> All = Build();

    private static IReadOnlyDictionary<string, MethodInfo> Build()
    {
        var methods = new Dictionary<string, MethodInfo>(StringComparer.Ordinal)
        {
            ["pnix.clr-meta.compiler-support.reader.v1/read-all"] = Method(typeof(ReaderAbi), nameof(ReaderAbi.ReadAll)),
            ["pnix.clr-meta.compiler-support.data.v1/kind-is?"] = Method(typeof(DataAbi), nameof(DataAbi.KindIs)),
            ["pnix.clr-meta.compiler-support.data.v1/count"] = Method(typeof(DataAbi), nameof(DataAbi.Count)),
            ["pnix.clr-meta.compiler-support.data.v1/nth"] = Method(typeof(DataAbi), nameof(DataAbi.Nth)),
            ["pnix.clr-meta.compiler-support.data.v1/symbol-name"] = Method(typeof(DataAbi), nameof(DataAbi.SymbolName)),
            ["pnix.clr-meta.compiler-support.data.v1/string-equal?"] = Method(typeof(DataAbi), nameof(DataAbi.StringEqual)),
            ["pnix.clr-meta.compiler-support.data.v1/env-new"] = Method(typeof(DataAbi), nameof(DataAbi.EnvNew)),
            ["pnix.clr-meta.compiler-support.data.v1/env-bind"] = Method(typeof(DataAbi), nameof(DataAbi.EnvBind)),
            ["pnix.clr-meta.compiler-support.data.v1/env-lookup"] = Method(typeof(DataAbi), nameof(DataAbi.EnvLookup)),
            ["pnix.clr-meta.compiler-support.data.v1/reject"] = Method(typeof(DataAbi), nameof(DataAbi.Reject)),
            ["pnix.clr-meta.compiler-support.pesink.v1/begin"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.Begin)),
            ["pnix.clr-meta.compiler-support.pesink.v1/define-constant"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.DefineConstant)),
            ["pnix.clr-meta.compiler-support.pesink.v1/define-method"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.DefineMethod)),
            ["pnix.clr-meta.compiler-support.pesink.v1/begin-initializer"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.BeginInitializer)),
            ["pnix.clr-meta.compiler-support.pesink.v1/end-initializer"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EndInitializer)),
            ["pnix.clr-meta.compiler-support.pesink.v1/begin-method"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.BeginMethod)),
            ["pnix.clr-meta.compiler-support.pesink.v1/end-method"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EndMethod)),
            ["pnix.clr-meta.compiler-support.pesink.v1/allocate-local"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.AllocateLocal)),
            ["pnix.clr-meta.compiler-support.pesink.v1/new-label"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.NewLabel)),
            ["pnix.clr-meta.compiler-support.pesink.v1/mark-label"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.MarkLabel)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-literal"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitLiteral)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-load-arg"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitLoadArg)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-load-local"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitLoadLocal)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-load-field"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitLoadField)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-store-local"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitStoreLocal)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-store-field"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitStoreField)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-call"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitCall)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-opcode"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitOpcode)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-branch-false"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitBranchFalse)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-branch"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitBranch)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-pop"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitPop)),
            ["pnix.clr-meta.compiler-support.pesink.v1/emit-ret"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.EmitRet)),
            ["pnix.clr-meta.compiler-support.pesink.v1/finish"] = Method(typeof(PeSinkAbi), nameof(PeSinkAbi.Finish)),
        };
        return methods;
    }

    private static MethodInfo Method(Type type, string name) =>
        type.GetMethod(name, BindingFlags.Public | BindingFlags.Static)
        ?? throw new InvalidOperationException($"missing support ABI method {type.FullName}/{name}");
}
