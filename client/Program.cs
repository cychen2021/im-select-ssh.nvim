using System.Buffers.Binary;
using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using MessagePack;

namespace ImSelectClient;

[MessagePackObject]
public class Request
{
    [Key("command")]
    public string Command { get; set; } = "";

    [Key("pin")]
    public string Pin { get; set; } = "";
}

[MessagePackObject]
public class Response
{
    [Key("success")]
    public bool Success { get; set; }

    [Key("error")]
    public string? Error { get; set; }
}

static class ImSelectRunner
{
    public static string GetCurrentIme(string imSelectPath)
    {
        var psi = new ProcessStartInfo(imSelectPath)
        {
            RedirectStandardOutput = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        using var proc = Process.Start(psi)
            ?? throw new Exception("failed to start im-select.exe");
        string output = proc.StandardOutput.ReadToEnd().Trim();
        proc.WaitForExit();
        if (proc.ExitCode != 0)
            throw new Exception($"im-select.exe exited with code {proc.ExitCode}");
        return output;
    }

    public static void SetIme(string imSelectPath, string imeCode)
    {
        var psi = new ProcessStartInfo(imSelectPath, imeCode)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        using var proc = Process.Start(psi)
            ?? throw new Exception("failed to start im-select.exe");
        proc.WaitForExit();
        if (proc.ExitCode != 0)
            throw new Exception($"im-select.exe set '{imeCode}' exited with code {proc.ExitCode}");
    }
}

class Program
{
    const int MaxFrameBytes = 64 * 1024;

    static readonly MessagePackSerializerOptions MpOptions =
        MessagePackSerializerOptions.Standard
            .WithSecurity(MessagePackSecurity.UntrustedData);

    static byte[] ReadFrame(NetworkStream stream)
    {
        byte[] lenBuf = new byte[4];
        stream.ReadExactly(lenBuf);
        int length = BinaryPrimitives.ReadInt32BigEndian(lenBuf);
        if (length <= 0 || length > MaxFrameBytes)
            throw new ProtocolViolationException($"invalid frame length: {length}");
        byte[] payload = new byte[length];
        stream.ReadExactly(payload);
        return payload;
    }

    static void WriteFrame(NetworkStream stream, byte[] payload)
    {
        byte[] lenBuf = new byte[4];
        BinaryPrimitives.WriteInt32BigEndian(lenBuf, payload.Length);
        stream.Write(lenBuf);
        stream.Write(payload);
        stream.Flush();
    }

    static Response HandleRequest(
        Request req,
        ref string? savedIme,
        string expectedPin,
        string imSelectPath,
        string defaultIme)
    {
        if (req.Pin != expectedPin)
            return new Response { Success = false, Error = "invalid pin" };

        try
        {
            switch (req.Command)
            {
                case "save_and_switch":
                    savedIme = ImSelectRunner.GetCurrentIme(imSelectPath);
                    ImSelectRunner.SetIme(imSelectPath, defaultIme);
                    return new Response { Success = true };

                case "restore":
                    if (savedIme != null)
                        ImSelectRunner.SetIme(imSelectPath, savedIme);
                    return new Response { Success = true };

                default:
                    return new Response
                    {
                        Success = false,
                        Error = $"unknown command: {req.Command}",
                    };
            }
        }
        catch (Exception ex)
        {
            return new Response { Success = false, Error = ex.Message };
        }
    }

    static void HandleClient(
        TcpClient client,
        ref string? savedIme,
        string expectedPin,
        string imSelectPath,
        string defaultIme)
    {
        using (client)
        {
            var stream = client.GetStream();
            stream.ReadTimeout = 10_000;
            stream.WriteTimeout = 10_000;

            Response response;
            try
            {
                byte[] payload = ReadFrame(stream);
                var request = MessagePackSerializer.Deserialize<Request>(payload, MpOptions);
                response = HandleRequest(request, ref savedIme, expectedPin, imSelectPath, defaultIme);
            }
            catch (Exception ex)
            {
                response = new Response { Success = false, Error = ex.Message };
            }

            try
            {
                byte[] respBytes = MessagePackSerializer.Serialize(response, MpOptions);
                WriteFrame(stream, respBytes);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Failed to send response: {ex.Message}");
            }
        }
    }

    static void PrintUsage()
    {
        Console.Error.WriteLine(
            "Usage: ImSelectClient --port <port> --pin <pin> [--im-select-path <path>] [--default-ime <code>]");
    }

    static void Main(string[] args)
    {
        int port = 0;
        string? pin = null;
        string imSelectPath = "im-select.exe";
        string defaultIme = "1033";

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--port" when i + 1 < args.Length:
                    if (!int.TryParse(args[++i], out port) || port <= 0 || port > 65535)
                    {
                        Console.Error.WriteLine($"Invalid port: {args[i]}");
                        Environment.Exit(1);
                    }
                    break;
                case "--pin" when i + 1 < args.Length:
                    pin = args[++i];
                    break;
                case "--im-select-path" when i + 1 < args.Length:
                    imSelectPath = args[++i];
                    break;
                case "--default-ime" when i + 1 < args.Length:
                    defaultIme = args[++i];
                    break;
                default:
                    Console.Error.WriteLine($"Unknown argument: {args[i]}");
                    PrintUsage();
                    Environment.Exit(1);
                    break;
            }
        }

        if (port == 0 || pin == null)
        {
            PrintUsage();
            Environment.Exit(1);
        }

        var listener = new TcpListener(IPAddress.Loopback, port);
        listener.Start();
        Console.WriteLine($"Listening on 127.0.0.1:{port}");

        using var cts = new CancellationTokenSource();
        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            cts.Cancel();
            listener.Stop();
        };

        string? savedIme = null;

        while (!cts.IsCancellationRequested)
        {
            TcpClient client;
            try
            {
                client = listener.AcceptTcpClient();
            }
            catch (SocketException) when (cts.IsCancellationRequested)
            {
                break;
            }

            HandleClient(client, ref savedIme, pin, imSelectPath, defaultIme);
        }

        Console.WriteLine("Shutting down");
    }
}
