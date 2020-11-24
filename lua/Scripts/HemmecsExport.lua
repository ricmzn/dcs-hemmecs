local client_instance = nil

package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
socket = require("socket")

log.write("HEMMECS.EXPORT", log.INFO, "Loaded")

function exportData(client)
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

function server()
    local tcp = socket.tcp()
    tcp:bind("127.0.0.1", 28561)
    local status, error = tcp:listen(1)
    if status == nil then
        log.write("HEMMECS.EXPORT", log.ERROR, "Could not listen for connections: "..error)
        return
    end
    tcp:setoption('keepalive', true)
    tcp:setoption('tcp-nodelay', true)
    tcp:settimeout(0)
    log.write("HEMMECS.EXPORT", log.INFO, "Listening for connections")
    local client = nil
    while true do
        if client == nil then
            local client, error = tcp:accept()
            if error ~= nil and error ~= "timeout" then
                log.write("HEMMECS.EXPORT", log.ERROR, "Failed to accept connection: "..error)
                break
            end
            if client ~= nil then
                log.write("HEMMECS.EXPORT", log.INFO, "Connected")
                client_instance = client
            end
        end
        coroutine.yield()
    end
end

serverCoroutine = coroutine.create(server)

function CoroutineResume(index, tCurrent)
	coroutine.resume(serverCoroutine, tCurrent)
	return coroutine.status(serverCoroutine) ~= "dead"
end

function LuaExportStart()
    LoCreateCoroutineActivity(0, 1.0, 1.0)
    log.write("HEMMECS.EXPORT", log.INFO, "Started")
end

function LuaExportStop()
    if client_instance ~= nil then
        client_instance:shutdown()
        log.write("HEMMECS.EXPORT", log.INFO, "Disconnected")
    end
    log.write("HEMMECS.EXPORT", log.INFO, "Stopped")
end

function LuaExportAfterNextFrame()
    if client_instance ~= nil then
        local status, error = exportData(client_instance)
        if error ~= nil then
            log.write("HEMMECS.EXPORT", log.ERROR, "Error sending message: "..error)
            client_instance:shutdown()
            client_instance = nil
        end
    end
end
