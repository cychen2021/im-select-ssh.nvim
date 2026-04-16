using System.Buffers.Binary;
using System.Net;
using System.Net.Sockets;
using ImSelectClient;
using MessagePack;
using Xunit;

namespace ImSelectClient.Tests;

public class IntegrationTests
{
    static readonly MessagePackSerializerOptions MpOpts =
        MessagePackSerializerOptions.Standard
            .WithSecurity(MessagePackSecurity.UntrustedData);

    private static (byte[] respBytes, Response resp) SendRequest(int port, Request req)
    {
        using var client = new TcpClient("127.0.0.1", port);
        var stream = client.GetStream();
        stream.ReadTimeout = 5000;
        stream.WriteTimeout = 5000;

        var reqBytes = MessagePackSerializer.Serialize(req, MpOpts);
        var lenBuf = new byte[4];
        BinaryPrimitives.WriteInt32BigEndian(lenBuf, reqBytes.Length);
        stream.Write(lenBuf);
        stream.Write(reqBytes);
        stream.Flush();

        var respLenBuf = new byte[4];
        stream.ReadExactly(respLenBuf);
        int respLen = (int)BinaryPrimitives.ReadUInt32BigEndian(respLenBuf);
        var respBuf = new byte[respLen];
        stream.ReadExactly(respBuf);
        var resp = MessagePackSerializer.Deserialize<Response>(respBuf, MpOpts);
        return (respBuf, resp);
    }

    [Fact]
    public async Task FullRoundtrip_InvalidPin()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        string? savedIme = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            Program.HandleClient(server, ref savedIme, "secret", "im-select.exe", "1033");
        });

        var (_, resp) = SendRequest(port, new Request { Command = "save_and_switch", Pin = "wrong" });
        await serverTask;
        listener.Stop();

        Assert.False(resp.Success);
        Assert.Equal("invalid pin", resp.Error);
    }

    [Fact]
    public async Task FullRoundtrip_UnknownCommand()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        string? savedIme = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            Program.HandleClient(server, ref savedIme, "pin", "im-select.exe", "1033");
        });

        var (_, resp) = SendRequest(port, new Request { Command = "invalid_cmd", Pin = "pin" });
        await serverTask;
        listener.Stop();

        Assert.False(resp.Success);
        Assert.Contains("unknown command", resp.Error!);
    }

    [Fact]
    public async Task FullRoundtrip_RestoreWithNoSavedIme()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        string? savedIme = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            Program.HandleClient(server, ref savedIme, "pin", "im-select.exe", "1033");
        });

        var (_, resp) = SendRequest(port, new Request { Command = "restore", Pin = "pin" });
        await serverTask;
        listener.Stop();

        Assert.True(resp.Success);
    }

    [Fact]
    public async Task FullRoundtrip_MultipleSequentialClients()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        string? savedIme = null;

        var serverTask = Task.Run(() =>
        {
            for (int i = 0; i < 3; i++)
            {
                using var server = listener.AcceptTcpClient();
                Program.HandleClient(server, ref savedIme, "pin", "im-select.exe", "1033");
            }
        });

        for (int i = 0; i < 3; i++)
        {
            var (_, resp) = SendRequest(port, new Request { Command = "restore", Pin = "pin" });
            Assert.True(resp.Success, $"request {i} should succeed");
        }

        await serverTask;
        listener.Stop();
    }
}
