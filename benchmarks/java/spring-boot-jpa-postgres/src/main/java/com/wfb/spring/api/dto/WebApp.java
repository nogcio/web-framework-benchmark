package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.List;
import java.util.Map;

@JsonInclude(JsonInclude.Include.NON_NULL)
public class WebApp {
    @JsonProperty("servlet")
    private List<ServletDef> servlet;

    @JsonProperty("servlet-mapping")
    private Map<String, String> servletMapping;

    @JsonProperty("taglib")
    private Taglib taglib;

    public List<ServletDef> getServlet() {
        return servlet;
    }

    public void setServlet(List<ServletDef> servlet) {
        this.servlet = servlet;
    }

    public Map<String, String> getServletMapping() {
        return servletMapping;
    }

    public void setServletMapping(Map<String, String> servletMapping) {
        this.servletMapping = servletMapping;
    }

    public Taglib getTaglib() {
        return taglib;
    }

    public void setTaglib(Taglib taglib) {
        this.taglib = taglib;
    }
}
