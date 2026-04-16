using ImSelectClient;
using Xunit;

namespace ImSelectClient.Tests;

public class HandleRequestTests
{
    [Fact]
    public void InvalidPin_ReturnsError()
    {
        string? savedIme = null;
        var req = new Request { Command = "save_and_switch", Pin = "wrong" };
        var resp = Program.HandleRequest(req, ref savedIme, "correct", "im-select.exe", "1033");
        Assert.False(resp.Success);
        Assert.Equal("invalid pin", resp.Error);
    }

    [Fact]
    public void InvalidPin_DoesNotModifySavedIme()
    {
        string? savedIme = "2052";
        var req = new Request { Command = "save_and_switch", Pin = "wrong" };
        Program.HandleRequest(req, ref savedIme, "correct", "im-select.exe", "1033");
        Assert.Equal("2052", savedIme);
    }

    [Fact]
    public void UnknownCommand_ReturnsError()
    {
        string? savedIme = null;
        var req = new Request { Command = "foobar", Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "im-select.exe", "1033");
        Assert.False(resp.Success);
        Assert.NotNull(resp.Error);
        Assert.Contains("unknown command", resp.Error);
        Assert.Contains("foobar", resp.Error);
    }

    [Fact]
    public void UnknownCommand_TruncatesLongCommandName()
    {
        string? savedIme = null;
        var longCmd = new string('x', 100);
        var req = new Request { Command = longCmd, Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "im-select.exe", "1033");
        Assert.False(resp.Success);
        Assert.NotNull(resp.Error);
        Assert.Contains("...", resp.Error);
        Assert.DoesNotContain(longCmd, resp.Error);
    }

    [Fact]
    public void RestoreWithNoSavedIme_Succeeds()
    {
        string? savedIme = null;
        var req = new Request { Command = "restore", Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "im-select.exe", "1033");
        Assert.True(resp.Success);
        Assert.Null(resp.Error);
    }

    [Fact]
    public void SaveAndSwitch_WhenImSelectFails_ReturnsErrorResponse()
    {
        string? savedIme = null;
        var req = new Request { Command = "save_and_switch", Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "/nonexistent/binary", "1033");
        Assert.False(resp.Success);
        Assert.NotNull(resp.Error);
    }

    [Fact]
    public void Restore_WhenImSelectFails_ReturnsErrorResponse()
    {
        string? savedIme = "1234";
        var req = new Request { Command = "restore", Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "/nonexistent/binary", "1033");
        Assert.False(resp.Success);
        Assert.NotNull(resp.Error);
    }

    [Fact]
    public void EmptyCommand_ReturnsUnknownCommandError()
    {
        string? savedIme = null;
        var req = new Request { Command = "", Pin = "pin" };
        var resp = Program.HandleRequest(req, ref savedIme, "pin", "im-select.exe", "1033");
        Assert.False(resp.Success);
        Assert.Contains("unknown command", resp.Error!);
    }
}
