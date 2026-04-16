using ImSelectClient;
using MessagePack;
using Xunit;

namespace ImSelectClient.Tests;

public class SerializationTests
{
    [Fact]
    public void Request_Roundtrip()
    {
        var req = new Request { Command = "save_and_switch", Pin = "123456" };
        var bytes = MessagePackSerializer.Serialize(req, Program.MpOptions);
        var decoded = MessagePackSerializer.Deserialize<Request>(bytes, Program.MpOptions);
        Assert.Equal("save_and_switch", decoded.Command);
        Assert.Equal("123456", decoded.Pin);
    }

    [Fact]
    public void Response_Success_Roundtrip()
    {
        var resp = new Response { Success = true, Error = null };
        var bytes = MessagePackSerializer.Serialize(resp, Program.MpOptions);
        var decoded = MessagePackSerializer.Deserialize<Response>(bytes, Program.MpOptions);
        Assert.True(decoded.Success);
        Assert.Null(decoded.Error);
    }

    [Fact]
    public void Response_Error_Roundtrip()
    {
        var resp = new Response { Success = false, Error = "bad pin" };
        var bytes = MessagePackSerializer.Serialize(resp, Program.MpOptions);
        var decoded = MessagePackSerializer.Deserialize<Response>(bytes, Program.MpOptions);
        Assert.False(decoded.Success);
        Assert.Equal("bad pin", decoded.Error);
    }

    [Fact]
    public void Request_EmptyFields()
    {
        var req = new Request { Command = "", Pin = "" };
        var bytes = MessagePackSerializer.Serialize(req, Program.MpOptions);
        var decoded = MessagePackSerializer.Deserialize<Request>(bytes, Program.MpOptions);
        Assert.Equal("", decoded.Command);
        Assert.Equal("", decoded.Pin);
    }

    [Fact]
    public void Request_RestoreCommand()
    {
        var req = new Request { Command = "restore", Pin = "abc" };
        var bytes = MessagePackSerializer.Serialize(req, Program.MpOptions);
        var decoded = MessagePackSerializer.Deserialize<Request>(bytes, Program.MpOptions);
        Assert.Equal("restore", decoded.Command);
        Assert.Equal("abc", decoded.Pin);
    }
}
