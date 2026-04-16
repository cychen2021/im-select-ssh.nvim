using System.Buffers.Binary;
using System.Net;
using System.Net.Sockets;
using ImSelectClient;
using Xunit;

namespace ImSelectClient.Tests;

public class FrameProtocolTests
{
    [Fact]
    public async Task WriteFrame_ReadFrame_Roundtrip()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        var payload = new byte[] { 1, 2, 3, 4, 5 };
        byte[]? received = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            var stream = server.GetStream();
            received = Program.ReadFrame(stream);
        });

        using (var client = new TcpClient("127.0.0.1", port))
        {
            var clientStream = client.GetStream();
            Program.WriteFrame(clientStream, payload);
        }

        await serverTask;
        listener.Stop();

        Assert.NotNull(received);
        Assert.Equal(payload, received);
    }

    [Fact]
    public async Task ReadFrame_ZeroLength_ThrowsProtocolViolation()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            var stream = server.GetStream();
            Assert.Throws<ProtocolViolationException>(() => Program.ReadFrame(stream));
        });

        using (var client = new TcpClient("127.0.0.1", port))
        {
            var clientStream = client.GetStream();
            var lenBuf = new byte[4];
            BinaryPrimitives.WriteInt32BigEndian(lenBuf, 0);
            clientStream.Write(lenBuf);
            clientStream.Flush();
        }

        await serverTask;
        listener.Stop();
    }

    [Fact]
    public async Task ReadFrame_OversizedLength_ThrowsProtocolViolation()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            var stream = server.GetStream();
            Assert.Throws<ProtocolViolationException>(() => Program.ReadFrame(stream));
        });

        using (var client = new TcpClient("127.0.0.1", port))
        {
            var clientStream = client.GetStream();
            var lenBuf = new byte[4];
            BinaryPrimitives.WriteInt32BigEndian(lenBuf, Program.MaxFrameBytes + 1);
            clientStream.Write(lenBuf);
            clientStream.Flush();
        }

        await serverTask;
        listener.Stop();
    }

    [Fact]
    public async Task WriteFrame_ProducesCorrectLengthPrefix()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        var payload = new byte[] { 0xAA, 0xBB, 0xCC };
        byte[]? rawData = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            var stream = server.GetStream();
            rawData = new byte[7];
            stream.ReadExactly(rawData);
        });

        using (var client = new TcpClient("127.0.0.1", port))
        {
            var clientStream = client.GetStream();
            Program.WriteFrame(clientStream, payload);
        }

        await serverTask;
        listener.Stop();

        Assert.NotNull(rawData);
        uint length = BinaryPrimitives.ReadUInt32BigEndian(rawData.AsSpan(0, 4));
        Assert.Equal(3u, length);
        Assert.Equal(payload, rawData[4..]);
    }

    [Fact]
    public async Task WriteFrame_ReadFrame_LargePayload()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;

        var payload = new byte[1024];
        new Random(42).NextBytes(payload);
        byte[]? received = null;

        var serverTask = Task.Run(() =>
        {
            using var server = listener.AcceptTcpClient();
            var stream = server.GetStream();
            received = Program.ReadFrame(stream);
        });

        using (var client = new TcpClient("127.0.0.1", port))
        {
            var clientStream = client.GetStream();
            Program.WriteFrame(clientStream, payload);
        }

        await serverTask;
        listener.Stop();

        Assert.NotNull(received);
        Assert.Equal(payload, received);
    }
}
