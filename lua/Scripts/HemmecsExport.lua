client = nil
server = nil

package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
socket = require("socket")
json = require("json")

function info(str)
    log.write("HEMMECS.EXPORT", log.INFO, str)
end

function error(str)
    log.write("HEMMECS.EXPORT", log.ERROR, str)
end

function exportData()
    local pitch, bank, yaw = LoGetADIPitchBankYaw()
    local data = json:encode({
        time = LoGetModelTime(),
        ias = LoGetIndicatedAirSpeed(),
        mach = LoGetMachNumber(),
        alt = LoGetAltitudeAboveSeaLevel(),
        rad_alt = LoGetAltitudeAboveGroundLevel(),
        pitch = pitch,
        bank = bank,
        yaw = yaw,
        aoa = LoGetAngleOfAttack(),
        g = LoGetAccelerationUnits(),
    })
    return client:send(data.."\n")
end

function disconnect()
    client:shutdown()
    client = nil
    info("Disconnected")
end

local previousLuaExportStart = LuaExportStart
local previousLuaExportStop = LuaExportStop
local previousLuaExportAfterNextFrame = LuaExportAfterNextFrame

function LuaExportStart()
    previousLuaExportStart()
    info("Started")
    server = socket.tcp()
    server:bind("127.0.0.1", 28561)
    local _, err = server:listen(1)
    if err ~= nil then
        error("Could not listen for connections: "..err)
        return
    end
    server:setoption('keepalive', true)
    server:setoption('tcp-nodelay', true)
    server:settimeout(0)
    info("Listening for connections")
end

function LuaExportStop()
    previousLuaExportStop()
    if client ~= nil then
        disconnect()
    end
    info("Stopped")
end

function LuaExportAfterNextFrame()
    previousLuaExportAfterNextFrame()
    if client == nil and server ~= nil then
        local err
        client, err = server:accept()
        if err ~= nil and err ~= "timeout" then
            error("Failed to accept connection: "..err)
        end
        if client ~= nil then
            info("Connected")
        end
    end
    if client ~= nil then
        local _, err = exportData()
        if err ~= nil then
            error("Error sending message: "..err)
            disconnect()
        end
    end
end

info("Loaded")
