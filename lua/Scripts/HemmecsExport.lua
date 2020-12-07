local client = nil
local server = nil

package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
local socket = require("socket")
local json = require("json")

local function log_info(str)
    log.write("HEMMECS.EXPORT", log.INFO, str)
end

local function log_error(str)
    log.write("HEMMECS.EXPORT", log.ERROR, str)
end

local function same_weapon(a, b)
    return a.level1 == b.level1
        and a.level2 == b.level2
        and a.level3 == b.level3
        and a.level4 == b.level4
end

local function exportData()
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

local function disconnect()
    client:shutdown()
    client = nil
    log_info("Disconnected")
end

local upstreamLuaExportStart = LuaExportStart
local upstreamLuaExportStop = LuaExportStop
local upstreamLuaExportAfterNextFrame = LuaExportAfterNextFrame

local function callUpstream(fn, name)
    if fn ~= nil then
        successful, err = pcall(fn)
        if not successful then
            log_error("Upstream error in "..name..": "..tostring(err))
        end
    end
end

function LuaExportStart()
    callUpstream(upstreamLuaExportStart, "LuaExportStart")
    log_info("Started")
    server = socket.tcp()
    server:bind("127.0.0.1", 28561)
    local _, err = server:listen(1)
    if err ~= nil then
        log_error("Could not listen for connections: "..err)
        return
    end
    server:setoption('keepalive', true)
    server:setoption('tcp-nodelay', true)
    server:settimeout(0)
    log_info("Listening for connections")
end

function LuaExportStop()
    callUpstream(upstreamLuaExportStop, "LuaExportStop")
    if client ~= nil then
        disconnect()
    end
    log_info("Stopped")
end

function LuaExportAfterNextFrame()
    callUpstream(upstreamLuaExportAfterNextFrame, "LuaExportAfterNextFrame")
    if client == nil and server ~= nil then
        local err
        client, err = server:accept()
        if err ~= nil and err ~= "timeout" then
            log_error("Failed to accept connection: "..err)
        end
        if client ~= nil then
            log_info("Connected")
        end
    end
    if client ~= nil then
        local _, err = exportData()
        if err ~= nil then
            log_error("Error sending message: "..err)
            disconnect()
        end
    end
end

log_info("Loaded")
