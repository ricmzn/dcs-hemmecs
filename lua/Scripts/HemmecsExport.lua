client = nil
server = nil

package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
socket = require("socket")

log.write("HEMMECS.EXPORT", log.INFO, "Loaded")

function exportData()
    local t = LoGetModelTime()
    local ias = LoGetIndicatedAirSpeed()
    local mach = LoGetMachNumber()
    local alt = LoGetAltitudeAboveSeaLevel()
    local radalt = LoGetAltitudeAboveGroundLevel()
    local pitch, roll, yaw = LoGetADIPitchBankYaw()
    local aoa = LoGetAngleOfAttack()
    local g = LoGetAccelerationUnits()
    return client:send(
        string.format(
            "t=%.2f,ias=%f,mach=%.2f,alt=%.2f,radalt=%.2f,pitch=%.2f,roll=%.2f,yaw=%.2f,aoa=%.2f,g.x=%.2f,g.y=%.2f,g.z=%.2f\n",
            t, ias, mach, alt, radalt, pitch, roll, yaw, aoa, g.x, g.y, g.z
        )
    )
end

function disconnect()
    client:shutdown()
    client = nil
    log.write("HEMMECS.EXPORT", log.INFO, "Disconnected")
end

function LuaExportStart()
    log.write("HEMMECS.EXPORT", log.INFO, "Started")
    server = socket.tcp()
    server:bind("127.0.0.1", 28561)
    local success, error = server:listen(1)
    if success == nil then
        log.write("HEMMECS.EXPORT", log.ERROR, "Could not listen for connections: "..error)
        return
    end
    server:setoption('keepalive', true)
    server:setoption('tcp-nodelay', true)
    server:settimeout(0)
    log.write("HEMMECS.EXPORT", log.INFO, "Listening for connections")
end

function LuaExportStop()
    if client ~= nil then
        disconnect(client)
    end
    log.write("HEMMECS.EXPORT", log.INFO, "Stopped")
end

function LuaExportAfterNextFrame()
    if client == nil and server ~= nil then
        local error
        client, error = server:accept()
        if error ~= nil and error ~= "timeout" then
            log.write("HEMMECS.EXPORT", log.ERROR, "Failed to accept connection: "..error)
        end
        if client ~= nil then
            log.write("HEMMECS.EXPORT", log.INFO, "Connected")
        end
    end
    if client ~= nil then
        local status, error = exportData()
        if error ~= nil then
            log.write("HEMMECS.EXPORT", log.ERROR, "Error sending message: "..error)
            disconnect()
        end
    end
end
