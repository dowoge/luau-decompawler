local fn = function()
    return "Final nonosolve" .. math.random(100000000, 999999999)
end

local value = (function(callback)
    while true do
        if not (function()
                return (function()
                    return 5, 6
                end)() == (function()
                    return 5
                end)()
            end)() then break end
        if (function()
                return (function()
                    return callback()
                end)()
            end)() then
            return (function(arg1_2)
                while true do
                    if not (function()
                            return (function()
                                return 5, 6
                            end)() == (function()
                                return 5
                            end)()
                        end)() then break end
                    if (function()
                            return (function()
                                return arg1_2()
                            end)()
                        end)() then
                        return (function(arg1_3)
                            while true do
                                if not (function()
                                        local n = 5
                                        return n == 5
                                    end)() then break end
                                if (function()
                                        return (function()
                                            return arg1_3()
                                        end)()
                                    end)() then
                                    return (function(arg1_4)
                                        while true do
                                            if not (function()
                                                    return (function()
                                                        return 5, 6
                                                    end)() == (function()
                                                        return 5
                                                    end)()
                                                end)() then break end
                                            if (function()
                                                    return (function()
                                                        return arg1_4()
                                                    end)()
                                                end)() then
                                                return (function(arg1_5)
                                                    while true do
                                                        if not (function()
                                                                return (function()
                                                                    return 5, 6
                                                                end)() == (function()
                                                                    return 5
                                                                end)()
                                                            end)() then break end
                                                        if arg1_5() then
                                                            return (function(arg1_6)
                                                                while true do
                                                                    if not (function()
                                                                            return (function()
                                                                                return 5, 6
                                                                            end)() == (function()
                                                                                return 5
                                                                            end)()
                                                                        end)() then break end
                                                                    if (function()
                                                                            return (function()
                                                                                return arg1_6()
                                                                            end)()
                                                                        end)() then
                                                                        return (function()
                                                                            return arg1_6()
                                                                        end)()
                                                                    end
                                                                    return (function()
                                                                        return "Fail T2"
                                                                    end)()
                                                                end
                                                                return (function()
                                                                    return "Fail T1"
                                                                end)()
                                                            end)(arg1_5)
                                                        end
                                                        return "Fail T2"
                                                    end
                                                    return "Fail T1"
                                                end)(arg1_4)
                                            end
                                            return (function()
                                                return "Fail T2"
                                            end)()
                                        end
                                        return (function()
                                            return "Fail T1"
                                        end)()
                                    end)(arg1_3)
                                end
                                return (function()
                                    return "Fail T2"
                                end)()
                            end
                            return (function()
                                return "Fail T1"
                            end)()
                        end)(arg1_2)
                    end
                    return (function()
                        return "Fail T2"
                    end)()
                end
                return (function()
                    return "Fail T1"
                end)()
            end)(callback)
        end
        return (function()
            return "Fail T2"
        end)()
    end
    return (function()
        return "Fail T1"
    end)()
end)(fn)
local callback = print
callback("out=", value)
