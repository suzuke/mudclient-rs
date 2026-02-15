-- Mock MUD Environment for Testing
local MockMud = {}
MockMud.__index = MockMud

function MockMud.new()
    local self = setmetatable({}, MockMud)
    self.sent = {}
    self.logs = {}
    self.timers = {}
    self.current_time = 0
    
    -- Bind methods to self to allow dot notation calls (mud.timer vs mud:timer)
    
    self.send = function(cmd)
        table.insert(self.sent, cmd)
    end

    self.echo = function(msg)
        table.insert(self.logs, msg)
    end
    
    self.print = function(msg)
        table.insert(self.logs, msg)
    end

    self.timer = function(seconds, callback_code)
        table.insert(self.timers, {
            trigger_time = self.current_time + seconds,
            code = callback_code
        })
        -- Sort by trigger time
        table.sort(self.timers, function(a, b) return a.trigger_time < b.trigger_time end)
    end

    return self
end

function MockMud:tick(seconds)
    self.current_time = self.current_time + seconds
    local remaining = {}
    local executed_count = 0
    
    local to_run = {}
    for _, t in ipairs(self.timers) do
        if t.trigger_time <= self.current_time then
            table.insert(to_run, t)
        else
            table.insert(remaining, t)
        end
    end
    
    self.timers = remaining
    
    for _, t in ipairs(to_run) do
        -- Mock execution
        -- Since code is likely "_G.MudUtils.exec_timer(...)", we load and run it
        local f, err = load(t.code)
        if f then
            local status, run_err = pcall(f)
            if not status then
                print("Mock Timer Error: " .. tostring(run_err))
            end
            executed_count = executed_count + 1
        else
             print("Mock Timer Load Error: " .. tostring(err) .. " | Code: " .. atosting(t.code))
        end
    end
    return executed_count
end

return MockMud
