local client = nil
local server = nil

package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
local socket = require("socket")
local json = require("json")

function info(str)
    log.write("HEMMECS.EXPORT", log.INFO, str)
end

function error(str)
    log.write("HEMMECS.EXPORT", log.ERROR, str)
end

function same_weapon(a, b)
    return a.level1 == b.level1
        and a.level2 == b.level2
        and a.level3 == b.level3
        and a.level4 == b.level4
end

function exportData()
    local cp_params = list_cockpit_params()
    local pitch, bank, yaw = LoGetADIPitchBankYaw()
    local weapons = nil
    local payload = LoGetPayloadInfo()
    local player = LoGetSelfData()
    if payload ~= nil then
        local selected = payload.Stations[payload.CurrentStation]
        if selected ~= nil then
            -- Manually count all weapons of the same type
            local count = 0
            for i, station in pairs(payload.Stations) do
                if same_weapon(station.weapon, selected.weapon) then
                    count = count + station.count
                end
            end
            weapons = {
                current = {
                    name = LoGetNameByType(selected.weapon.level1, selected.weapon.level2, selected.weapon.level3, selected.weapon.level4),
                    count = count,
                },
                shells = payload.Cannon.shells,
            }
        else
            weapons = {
                shells = payload.Cannon.shells,
            }
        end
    end
    local data = json:encode({
        cp_params = cp_params,
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
        cam = LoGetCameraPosition(),
        engine_data = LoGetEngineInfo(),
        unit = player.Name,
        weapons = weapons
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
