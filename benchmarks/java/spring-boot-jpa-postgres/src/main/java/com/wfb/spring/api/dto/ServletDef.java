package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.Map;

@JsonInclude(JsonInclude.Include.NON_NULL)
public class ServletDef {
    @JsonProperty("servlet-name")
    private String servletName;

    @JsonProperty("servlet-class")
    private String servletClass;

    @JsonProperty("init-param")
    private Map<String, Object> initParam;

    public String getServletName() {
        return servletName;
    }

    public void setServletName(String servletName) {
        this.servletName = servletName;
    }

    public String getServletClass() {
        return servletClass;
    }

    public void setServletClass(String servletClass) {
        this.servletClass = servletClass;
    }

    public Map<String, Object> getInitParam() {
        return initParam;
    }

    public void setInitParam(Map<String, Object> initParam) {
        this.initParam = initParam;
    }
}
